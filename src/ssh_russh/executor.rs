use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use bytes::BytesMut;
use once_cell::sync::Lazy;
use russh::{
    client::{self, Msg},
    Channel, ChannelId, ChannelMsg, Sig,
};
use tokio::sync::Mutex;

use crate::executor::{LinuxExecutor, LinuxProcess, LinuxProcessConfiguration, LinuxProcessError, LinuxProcessOutput};

use super::RusshLinux;

pub(super) static EXECUTOR_BUFFERS: Lazy<Arc<RwLock<HashMap<ChannelId, BytesMut>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct RusshLinuxProcess {
    pub(super) channel_id: ChannelId,
    pub(super) channel_mutex: Arc<Mutex<Channel<Msg>>>,
}

#[async_trait]
impl LinuxProcess for RusshLinuxProcess {
    fn id(&self) -> Option<u32> {
        None
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

    async fn send_kill_request(&mut self) -> Result<(), LinuxProcessError> {
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
        process_configuration: LinuxProcessConfiguration,
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

        Ok(RusshLinuxProcess {
            channel_id: channel.id(),
            channel_mutex: Arc::new(Mutex::new(channel)),
        })
    }

    async fn execute(
        &self,
        process_configuration: LinuxProcessConfiguration,
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
    process_configuration: LinuxProcessConfiguration,
) -> Result<(), russh::Error> {
    // 1. apply env vars
    for (key, value) in process_configuration.envs {
        channel.set_env(true, key, value).await?;
    }
    // 2. cd if necessary
    if let Some(path) = process_configuration.working_dir {
        channel.exec(true, format!("cd {}", path.to_str().unwrap())).await?;
        loop {
            match channel.wait().await {
                None => break,
                Some(_) => {}
            }
        }
    }
    // 3. construct full command
    let mut command = process_configuration.program;
    if process_configuration.args.len() > 0 {
        command += " ";
        command += process_configuration.args.join(" ").as_str();
    }
    // 4. issue command
    channel.exec(true, command).await?;

    Ok(())
}
