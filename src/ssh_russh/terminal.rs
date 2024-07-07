use std::sync::Arc;

use async_trait::async_trait;
use russh::{client::Msg, Channel, ChannelId};
use tokio::sync::Mutex;

use crate::terminal::{LinuxTerminal, LinuxTerminalError, LinuxTerminalEventReceiver, LinuxTerminalLauncher};

use super::{
    event_receiver::{RusshGlobalReceiver, DHS},
    RusshLinux,
};

pub struct RusshLinuxTerminal {
    #[allow(unused)]
    channel_mutex: Arc<Mutex<Channel<Msg>>>,
    channel_id: ChannelId,
}

#[async_trait]
impl LinuxTerminal for RusshLinuxTerminal {
    fn supports_event_receiver(&self) -> bool {
        true
    }

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
