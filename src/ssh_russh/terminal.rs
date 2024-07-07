use std::sync::Arc;

use async_trait::async_trait;
use russh::{client::Msg, Channel, ChannelId, ChannelMsg, Sig};
use tokio::{io::AsyncWriteExt, sync::Mutex};

use crate::terminal::{
    LinuxTerminal, LinuxTerminalError, LinuxTerminalEvent, LinuxTerminalEventReceiver, LinuxTerminalLauncher,
};

use super::{
    event_receiver::{conv_sig_to_str, RusshGlobalReceiver, DHS},
    RusshLinux,
};

pub struct RusshLinuxTerminal {
    #[allow(unused)]
    channel_mutex: Arc<Mutex<Channel<Msg>>>,
    channel_id: ChannelId,
}

#[async_trait]
impl LinuxTerminal for RusshLinuxTerminal {
    async fn register_event_receiver<R>(&self, receiver: R) -> Result<(), LinuxTerminalError>
    where
        R: LinuxTerminalEventReceiver + 'static,
    {
        let read_ref = DHS.read().await;
        if let Some(_) = read_ref.get(&self.channel_id) {
            return Err(LinuxTerminalError::EventReceiverAlreadyExists);
        }
        drop(read_ref);

        let mut write_ref = DHS.write().await;
        write_ref.insert(self.channel_id, Box::new(receiver));
        drop(write_ref);

        Ok(())
    }

    async fn unregister_event_receiver(&self) -> Result<(), LinuxTerminalError> {
        let read_ref = DHS.read().await;
        if let None = read_ref.get(&self.channel_id) {
            return Err(LinuxTerminalError::EventReceiverMissing);
        }
        drop(read_ref);

        let mut write_ref = DHS.write().await;
        write_ref.remove(&self.channel_id);
        drop(write_ref);

        Ok(())
    }

    async fn run(&self, command: String) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel.exec(true, command).await.map_err(LinuxTerminalError::other)
    }

    async fn set_env_var(&self, name: String, value: String) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .set_env(true, name, value)
            .await
            .map_err(LinuxTerminalError::other)
    }

    async fn send_eof(&self) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel.eof().await.map_err(LinuxTerminalError::other)
    }

    async fn send_signal(&self, signal: String) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .signal(Sig::Custom(signal))
            .await
            .map_err(LinuxTerminalError::other)
    }

    async fn send_input(&self, input: &[u8], ext: Option<u32>) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel
            .make_writer_ext(ext)
            .write_all(input)
            .await
            .map_err(LinuxTerminalError::other)
    }

    async fn await_next_event(&self) -> Option<LinuxTerminalEvent> {
        let mut channel = self.channel_mutex.lock().await;

        loop {
            match channel.wait().await {
                None => {
                    return None; // no event to be received
                }
                Some(ChannelMsg::Eof) => return Some(LinuxTerminalEvent::EOFReceived),
                Some(ChannelMsg::Data { data }) => {
                    let vec: Vec<u8> = data.as_ref().into();
                    return Some(LinuxTerminalEvent::DataReceived { data: vec });
                }
                Some(ChannelMsg::ExtendedData { data, ext }) => {
                    let vec: Vec<u8> = data.as_ref().into();
                    return Some(LinuxTerminalEvent::ExtendedDataReceived {
                        ext,
                        extended_data: vec,
                    });
                }
                Some(ChannelMsg::XonXoff { client_can_do }) => {
                    return Some(LinuxTerminalEvent::XonXoffAbilityReceived {
                        can_perform_xon_xoff: client_can_do,
                    })
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    return Some(LinuxTerminalEvent::ProcessExitedNormally { exit_status });
                }
                Some(ChannelMsg::ExitSignal {
                    signal_name,
                    core_dumped,
                    error_message,
                    lang_tag,
                }) => {
                    return Some(LinuxTerminalEvent::ProcessExitedAfterSignal {
                        signal: conv_sig_to_str(signal_name),
                        core_dumped,
                        error_message,
                        lang_tag,
                    });
                }
                Some(ChannelMsg::WindowAdjusted { new_size }) => {
                    return Some(LinuxTerminalEvent::WindowAdjusted { new_size });
                }
                Some(ChannelMsg::Success) => {
                    return Some(LinuxTerminalEvent::QueuedOperationSucceeded);
                }
                Some(ChannelMsg::Failure) => {
                    return Some(LinuxTerminalEvent::QueuedOperationFailed);
                }
                Some(_) => {} // an event that isn't supposed to be received by a terminal, but rather by a global receiver
            }
        }
    }

    async fn quit(&self) -> Result<(), LinuxTerminalError> {
        let channel = self.channel_mutex.lock().await;
        channel.close().await.map_err(LinuxTerminalError::other)?;
        DHS.write().await.remove(&self.channel_id);
        Ok(())
    }
}

#[async_trait]
impl<R> LinuxTerminalLauncher for RusshLinux<R>
where
    R: RusshGlobalReceiver,
{
    async fn launch_terminal_noninteractive(&self) -> Result<RusshLinuxTerminal, LinuxTerminalError> {
        let handle = self.handle_mutex.lock().await;
        let channel = handle.channel_open_session().await.map_err(LinuxTerminalError::other)?;
        let channel_id = channel.id();
        Ok(RusshLinuxTerminal {
            channel_mutex: Arc::new(Mutex::new(channel)),
            channel_id,
        })
    }

    async fn launch_terminal_interactive(
        &self,
        terminal: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
    ) -> Result<RusshLinuxTerminal, LinuxTerminalError> {
        let handle = self.handle_mutex.lock().await;
        let channel = handle.channel_open_session().await.map_err(LinuxTerminalError::other)?;
        channel
            .request_pty(true, &terminal, col_width, row_height, pix_width, pix_height, &[])
            .await
            .map_err(LinuxTerminalError::other)?;
        let channel_id = channel.id();
        Ok(RusshLinuxTerminal {
            channel_mutex: Arc::new(Mutex::new(channel)),
            channel_id,
        })
    }
}
