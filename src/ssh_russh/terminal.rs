use std::{io, sync::Arc};

use async_trait::async_trait;
use russh::{client::Msg, Channel, ChannelId};
use tokio::sync::Mutex;

use crate::terminal::{LinuxTerminal, LinuxTerminalLauncher};

use super::{event_receiver::RusshGlobalReceiver, RusshLinux};

pub struct RusshLinuxTerminal {
    channel_mutex: Arc<Mutex<Channel<Msg>>>,
    channel_id: ChannelId,
}

impl LinuxTerminal for RusshLinuxTerminal {}

#[async_trait]
impl<R> LinuxTerminalLauncher for RusshLinux<R>
where
    R: RusshGlobalReceiver,
{
    async fn launch_terminal_noninteractive(&self) -> Result<RusshLinuxTerminal, io::Error> {
        let handle = self.handle_mutex.lock().await;
        let channel = handle.channel_open_session().await.map_err(io::Error::other)?;
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
    ) -> Result<RusshLinuxTerminal, io::Error> {
        let handle = self.handle_mutex.lock().await;
        let channel = handle.channel_open_session().await.map_err(io::Error::other)?;
        channel
            .request_pty(true, &terminal, col_width, row_height, pix_width, pix_height, &[])
            .await
            .map_err(io::Error::other)?;
        let channel_id = channel.id();
        Ok(RusshLinuxTerminal {
            channel_mutex: Arc::new(Mutex::new(channel)),
            channel_id,
        })
    }
}
