use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use russh::{
    client::{self, Msg},
    Channel, ChannelId, ChannelMsg, Sig,
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::executor::{LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError, LinuxProcessOutput};

use super::RusshLinux;

pub(super) static EXECUTOR_BUFFERS: Lazy<Arc<RwLock<HashMap<ChannelId, BytesMut>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

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

    async fn await_exit(&mut self) -> Result<Option<i64>, LinuxProcessError> {
        let mut channel = self.channel_mutex.lock().await;
        let mut status_code: Option<i64> = None;

        loop {
            match channel.wait().await {
                None => break,
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    status_code = Some(exit_status.into());
                }
                Some(_) => {}
            }
        }

        Ok(status_code)
    }

    async fn await_exit_with_output(mut self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let status_code = self.await_exit().await?;
        let stdout: Vec<u8> = match EXECUTOR_BUFFERS
            .read()
            .expect("Executor buffers RWLock was poisoned!")
            .get(&self.channel_id)
        {
            Some(buf) => buf.as_ref().into(),
            None => Vec::new(),
        };

        Ok(LinuxProcessOutput {
            stdout,
            stderr: Vec::new(),
            status_code,
        })
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

#[async_trait]
impl<H> LinuxExecutor for RusshLinux<H>
where
    H: client::Handler,
{
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<RusshLinuxProcess, LinuxProcessError> {
        let handle = self.handle_mutex.lock().await;
        let mut channel = handle
            .channel_open_session()
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        apply_process_configuration(&mut channel, process_configuration)
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        EXECUTOR_BUFFERS
            .write()
            .expect("Executor buffers RWLock was poisoned!")
            .insert(channel.id(), BytesMut::new());

        if process_configuration.redirect_stdin {
            channel
                .request_pty(
                    false,
                    &self.pty_options.terminal,
                    self.pty_options.col_width,
                    self.pty_options.row_height,
                    self.pty_options.pix_width,
                    self.pty_options.pix_height,
                    &self.pty_options.terminal_modes,
                )
                .await
                .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        }

        let stdin = Box::pin(channel.make_writer());

        Ok(RusshLinuxProcess {
            channel_id: channel.id(),
            channel_mutex: Arc::new(Mutex::new(channel)),
            stdin: Some(stdin),
        })
    }

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError> {
        let process = self.begin_execute(process_configuration).await?;
        let mut channel = process.channel_mutex.lock().await;

        let mut status_code: Option<i64> = None;
        loop {
            match channel.wait().await {
                None => break,
                Some(ChannelMsg::ExitStatus { exit_status }) => status_code = Some(exit_status.into()),
                Some(_) => {}
            }
        }

        let stdout: Vec<u8> = match EXECUTOR_BUFFERS
            .read()
            .expect("Executor buffers RWLock was poisoned!")
            .get(&process.channel_id)
        {
            Some(buf) => buf.as_ref().into(),
            None => Vec::new(),
        };

        Ok(LinuxProcessOutput {
            stdout,
            stderr: Vec::new(),
            status_code,
        })
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

    // 4. issue command
    channel.exec(true, command).await?;

    Ok(())
}
