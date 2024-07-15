use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use bytes::BytesMut;
use dashmap::DashMap;
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
    FinishedLinuxProcessOutput, LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError,
    LinuxProcessOutput,
};

use super::RusshLinux;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct InternalId {
    pub channel_id: ChannelId,
    pub instance_id: u16,
}

pub(super) static STDOUT_BUFFERS: Lazy<Arc<DashMap<InternalId, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));
pub(super) static STDERR_BUFFERS: Lazy<Arc<DashMap<InternalId, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));
pub(super) static STDEXT_BUFFERS: Lazy<Arc<DashMap<StdextKey, BytesMut>>> = Lazy::new(|| Arc::new(DashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct StdextKey {
    pub channel_id: ChannelId,
    pub instance_id: u16,
    pub ext: u32,
}

struct RusshLinuxProcess {
    pub(super) internal_id: InternalId,
    pub(super) channel_mutex: Arc<Mutex<Channel<Msg>>>,
    pub(super) stdin: Option<Pin<Box<dyn AsyncWrite + Send>>>,
}

#[async_trait]
impl LinuxProcess for RusshLinuxProcess {
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

    fn get_current_output(&self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        Ok(fetch_process_output(&self.internal_id))
    }

    async fn await_exit(self: Box<Self>) -> Result<Option<i64>, LinuxProcessError> {
        let mut channel = self.channel_mutex.lock().await;
        let status = await_process_exit(&mut channel).await;
        Ok(status)
    }

    async fn await_exit_with_output(self: Box<Self>) -> Result<FinishedLinuxProcessOutput, LinuxProcessError> {
        let mut channel = self.channel_mutex.lock().await;
        let status_code = await_process_exit(&mut channel).await;
        drop(channel);
        let output = self.get_current_output()?;
        Ok(FinishedLinuxProcessOutput::join(output, status_code))
    }

    async fn send_kill_request(&mut self) -> Result<(), LinuxProcessError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .signal(Sig::INT)
            .await
            .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
        Ok(())
    }
}

impl Drop for RusshLinuxProcess {
    fn drop(&mut self) {
        STDOUT_BUFFERS.remove(&self.internal_id);
        STDERR_BUFFERS.remove(&self.internal_id);
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
    ) -> Result<FinishedLinuxProcessOutput, LinuxProcessError> {
        let process = begin_execute_internal(&self, process_configuration).await?;
        let mut channel = process.channel_mutex.lock().await;
        let status_code = await_process_exit(&mut channel).await;
        let output = fetch_process_output(&InternalId {
            channel_id: channel.id(),
            instance_id: self.id,
        });
        Ok(FinishedLinuxProcessOutput::join(output, status_code))
    }
}

async fn begin_execute_internal<H: client::Handler>(
    instance: &RusshLinux<H>,
    process_configuration: &LinuxProcessConfiguration,
) -> Result<RusshLinuxProcess, LinuxProcessError> {
    let handle = instance.handle_mutex.lock().await;
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;
    apply_process_configuration(&mut channel, process_configuration)
        .await
        .map_err(|err| LinuxProcessError::Other(Box::new(err)))?;

    if process_configuration.redirect_stdout {
        STDOUT_BUFFERS.insert(
            InternalId {
                channel_id: channel.id(),
                instance_id: instance.id,
            },
            BytesMut::new(),
        );
    }
    if process_configuration.redirect_stderr {
        STDERR_BUFFERS.insert(
            InternalId {
                channel_id: channel.id(),
                instance_id: instance.id,
            },
            BytesMut::new(),
        );
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
        internal_id: InternalId {
            channel_id: channel.id(),
            instance_id: instance.id,
        },
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

fn fetch_process_output(internal_id: &InternalId) -> LinuxProcessOutput {
    let stdout = match STDOUT_BUFFERS.get(internal_id) {
        Some(buf) => buf.to_vec(),
        None => Vec::new(),
    };

    let stderr = match STDERR_BUFFERS.get(internal_id) {
        Some(buf) => buf.to_vec(),
        None => Vec::new(),
    };

    let stdout_extended = STDEXT_BUFFERS
        .iter()
        .filter(|entry| {
            entry.key().channel_id == internal_id.channel_id && entry.key().instance_id == internal_id.instance_id
        })
        .map(|entry| (entry.key().ext, entry.value().to_vec()))
        .collect();

    LinuxProcessOutput {
        stdout,
        stderr,
        stdout_extended,
    }
}

async fn apply_process_configuration(
    channel: &mut Channel<Msg>,
    process_configuration: &LinuxProcessConfiguration,
) -> Result<String, russh::Error> {
    let (command, pid_file) = process_configuration.desugar_to_shell_command();
    channel.exec(true, command).await?;

    Ok(pid_file)
}
