use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use bytes::BytesMut;
use once_cell::sync::Lazy;
use russh::{
    client::{self, Msg},
    Channel, ChannelId, ChannelMsg, Sig,
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::executor::{
    LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError, LinuxProcessOutput,
    LinuxProcessPartialOutput,
};

use super::RusshLinux;

pub(super) static STDOUT_BUFFERS: Lazy<Arc<RwLock<HashMap<ChannelId, BytesMut>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
pub(super) static STDERR_BUFFERS: Lazy<Arc<RwLock<HashMap<ChannelId, BytesMut>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
pub(super) static STDEXT_BUFFERS: Lazy<Arc<RwLock<Vec<StdextEntry>>>> = Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

pub(super) struct StdextEntry {
    pub channel_id: ChannelId,
    pub ext: u32,
    pub buffer: BytesMut,
}

pub struct RusshLinuxProcess<'a> {
    pub(super) channel_id: ChannelId,
    pub(super) channel_mutex: Arc<Mutex<Channel<Msg>>>,
    pub(super) stdin: Option<Pin<Box<dyn AsyncWrite + Send + 'a>>>,
}

#[async_trait]
impl<'a> LinuxProcess for RusshLinuxProcess<'a> {
    fn id(&self) -> Option<u32> {
        None
    }

    async fn write_to_stdin(&mut self, data: &[u8]) -> Result<usize, LinuxProcessError> {
        let stdin_ref = self.stdin.as_mut().ok_or(LinuxProcessError::StdinNotPiped)?;
        stdin_ref
            .write(data)
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))
    }

    async fn close_stdin(&mut self) -> Result<(), LinuxProcessError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .eof()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        Ok(())
    }

    fn get_partial_output(&self) -> Result<LinuxProcessPartialOutput, LinuxProcessError> {
        Ok(fetch_partial_process_output(&self.channel_id))
    }

    async fn await_exit(&mut self) -> Result<Option<i64>, LinuxProcessError> {
        let mut channel = self.channel_mutex.lock().await;
        let status = await_process_exit(&mut channel).await;
        Ok(status)
    }

    async fn await_exit_with_output(mut self: Box<Self>) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let mut channel = self.channel_mutex.lock().await;
        let status_code = await_process_exit(&mut channel).await;
        Ok(fetch_process_output(&self.channel_id, status_code))
    }

    async fn begin_kill(&mut self) -> Result<(), LinuxProcessError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .signal(Sig::KILL)
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        Ok(())
    }
}

impl Drop for RusshLinuxProcess<'_> {
    fn drop(&mut self) {
        cleanup_buffers(&self.channel_id);
    }
}

#[async_trait]
impl<H> LinuxExecutor for RusshLinux<H>
where
    H: client::Handler,
{
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<Box<dyn LinuxProcess>, LinuxProcessError> {
        let process = begin_execute_internal(&self, process_configuration).await?;
        Ok(Box::new(process))
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let process = begin_execute_internal(&self, process_configuration).await?;
        let mut channel = process.channel_mutex.lock().await;
        let status_code = await_process_exit(&mut channel).await;
        Ok(fetch_process_output(&channel.id(), status_code))
    }
}

async fn begin_execute_internal<'a, H: client::Handler>(
    instance: &RusshLinux<H>,
    process_configuration: &LinuxProcessConfiguration,
) -> Result<RusshLinuxProcess<'a>, LinuxProcessError> {
    let handle = instance.handle_mutex.lock().await;
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
    apply_process_configuration(&mut channel, process_configuration)
        .await
        .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;

    if process_configuration.redirect_stdout {
        STDOUT_BUFFERS
            .write()
            .expect("Stdout rwlock was poisoned!")
            .insert(channel.id(), BytesMut::new());
    }
    if process_configuration.redirect_stderr {
        STDERR_BUFFERS
            .write()
            .expect("Stderr rwlock was poisoned!")
            .insert(channel.id(), BytesMut::new());
    }

    if process_configuration.redirect_stdin {
        channel
            .request_pty(
                false,
                &instance.pty_options.terminal,
                instance.pty_options.col_width,
                instance.pty_options.row_height,
                instance.pty_options.pix_width,
                instance.pty_options.pix_height,
                &instance.pty_options.terminal_modes,
            )
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
    }

    let stdin = Box::pin(channel.make_writer()) as Pin<Box<dyn AsyncWrite + Send>>;
    let stdin_option = match process_configuration.redirect_stdin {
        true => Some(stdin),
        false => None,
    };

    Ok(RusshLinuxProcess {
        channel_id: channel.id(),
        channel_mutex: Arc::new(Mutex::new(channel)),
        stdin: stdin_option,
    })
}

async fn await_process_exit(channel: &mut Channel<Msg>) -> Option<i64> {
    let mut status_code = None;

    loop {
        match channel.wait().await {
            None => break,
            Some(ChannelMsg::ExitStatus { exit_status }) => {
                status_code = Some(exit_status.into());
            }
            Some(_) => {}
        }
    }

    status_code
}

fn cleanup_buffers(channel_id: &ChannelId) {
    STDOUT_BUFFERS
        .write()
        .expect("Stdout rwlock was poisoned!")
        .remove(channel_id);
    STDERR_BUFFERS
        .write()
        .expect("Stderr rwlock was poisoned!")
        .remove(channel_id);
}

fn fetch_process_output(channel_id: &ChannelId, status_code: Option<i64>) -> LinuxProcessOutput {
    let partial_output = fetch_partial_process_output(channel_id);
    cleanup_buffers(channel_id);

    LinuxProcessOutput {
        stdout: partial_output.stdout,
        stderr: partial_output.stderr,
        stdout_extended: partial_output.stdout_extended,
        status_code,
    }
}

fn fetch_partial_process_output(channel_id: &ChannelId) -> LinuxProcessPartialOutput {
    let stdout = match STDOUT_BUFFERS
        .read()
        .expect("Stdout rwlock was poisoned!")
        .get(&channel_id)
    {
        Some(buf) => buf.to_vec(),
        None => Vec::new(),
    };

    let stderr = match STDERR_BUFFERS
        .read()
        .expect("Stderr rwlock was poisoned!")
        .get(&channel_id)
    {
        Some(buf) => buf.to_vec(),
        None => Vec::new(),
    };

    let stdout_extended = STDEXT_BUFFERS
        .read()
        .expect("Stdext rwlock was poisoned!")
        .iter()
        .filter(|entry| entry.channel_id == *channel_id)
        .map(|entry| (entry.ext, entry.buffer.as_ref().to_vec()))
        .collect();

    LinuxProcessPartialOutput {
        stdout,
        stderr,
        stdout_extended,
    }
}

async fn apply_process_configuration(
    channel: &mut Channel<Msg>,
    process_configuration: &LinuxProcessConfiguration,
) -> Result<(), russh::Error> {
    let mut env_string = String::new();
    for (key, value) in &process_configuration.envs {
        env_string.push_str(format!("{}={} ", key, value).as_str());
    }

    let mut command = process_configuration.program.clone();
    if process_configuration.args.len() > 0 {
        command += " ";
        command += process_configuration.args.join(" ").as_str();
    }
    if env_string.len() > 0 {
        command = env_string + command.as_str();
    }

    if let Some(path) = &process_configuration.working_dir {
        command = format!("(cd {} && {})", path.to_str().unwrap(), command);
    }

    channel.exec(true, command).await?;

    Ok(())
}
