pub mod connection;
#[cfg(feature = "executor")]
mod executor;
#[cfg(feature = "filesystem")]
mod filesystem;
#[cfg(feature = "network")]
mod network;

use std::sync::Arc;

use async_trait::async_trait;
#[cfg(feature = "executor")]
use executor::{InternalId, StdextKey, STDERR_BUFFERS, STDEXT_BUFFERS, STDOUT_BUFFERS};

use russh::{
    client::{self, DisconnectReason, Msg, Session},
    Channel, ChannelId, ChannelOpenFailure, Pty, Sig,
};
use russh_keys::key::PublicKey;
use tokio::sync::Mutex;

#[allow(unused)]
pub struct RusshLinux<H>
where
    H: client::Handler,
{
    id: u16,
    pty_options: RusshPtyOptions,
    handle_mutex: Arc<Mutex<client::Handle<WrappingHandler<H>>>>,
    fs_channel_mutex: Arc<Mutex<Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshPtyOptions {
    pub terminal: String,
    pub col_width: u32,
    pub row_height: u32,
    pub pix_width: u32,
    pub pix_height: u32,
    pub terminal_modes: Vec<(Pty, u32)>,
}

pub(super) struct WrappingHandler<H>
where
    H: client::Handler,
{
    pub inner: H,
    #[allow(unused)]
    pub instance_id: u16,
}

#[async_trait]
impl<H> client::Handler for WrappingHandler<H>
where
    H: client::Handler,
{
    type Error = H::Error;

    #[allow(unused_variables)]
    async fn auth_banner(&mut self, banner: &str, session: &mut Session) -> Result<(), Self::Error> {
        self.inner.auth_banner(banner, session).await
    }

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        self.inner.check_server_key(server_public_key).await
    }

    async fn channel_open_confirmation(
        &mut self,
        id: ChannelId,
        max_packet_size: u32,
        window_size: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .channel_open_confirmation(id, max_packet_size, window_size, session)
            .await
    }

    async fn channel_success(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        self.inner.channel_success(channel, session).await
    }

    async fn channel_failure(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        self.inner.channel_failure(channel, session).await
    }

    async fn channel_close(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        self.inner.channel_close(channel, session).await
    }

    async fn channel_eof(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        self.inner.channel_eof(channel, session).await
    }

    async fn channel_open_failure(
        &mut self,
        channel: ChannelId,
        reason: ChannelOpenFailure,
        description: &str,
        language: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .channel_open_failure(channel, reason, description, language, session)
            .await
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .server_channel_open_forwarded_tcpip(
                channel,
                connected_address,
                connected_port,
                originator_address,
                originator_port,
                session,
            )
            .await
    }

    async fn server_channel_open_agent_forward(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.server_channel_open_agent_forward(channel, session).await
    }

    fn server_channel_handle_unknown(&self, channel: ChannelId, channel_type: &[u8]) -> bool {
        self.inner.server_channel_handle_unknown(channel, channel_type)
    }

    async fn server_channel_open_session(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.server_channel_open_session(channel, session).await
    }

    async fn server_channel_open_direct_tcpip(
        &mut self,
        channel: ChannelId,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .server_channel_open_direct_tcpip(
                channel,
                host_to_connect,
                port_to_connect,
                originator_address,
                originator_port,
                session,
            )
            .await
    }

    #[allow(unused_variables)]
    async fn server_channel_open_x11(
        &mut self,
        channel: Channel<Msg>,
        originator_address: &str,
        originator_port: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .server_channel_open_x11(channel, originator_address, originator_port, session)
            .await
    }

    #[allow(unused_variables)]
    async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) -> Result<(), Self::Error> {
        #[cfg(feature = "executor")]
        if let Some(mut buf) = STDOUT_BUFFERS.get_mut(&InternalId {
            channel_id: channel,
            instance_id: self.instance_id,
        }) {
            buf.extend(data);
        }

        self.inner.data(channel, data, session).await
    }

    async fn extended_data(
        &mut self,
        channel: ChannelId,
        ext: u32,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "executor")]
        if ext == 1 {
            // ext 1 is stderr according to SSH spec
            if let Some(mut buf) = STDERR_BUFFERS.get_mut(&InternalId {
                channel_id: channel,
                instance_id: self.instance_id,
            }) {
                buf.extend(data);
            }
        } else {
            let stdext_key = StdextKey {
                channel_id: channel,
                instance_id: self.instance_id,
                ext,
            };
            match STDEXT_BUFFERS.get_mut(&stdext_key) {
                Some(mut existing_entry) => {
                    existing_entry.value_mut().extend(data);
                }
                None => {
                    STDEXT_BUFFERS.insert(stdext_key, data.into());
                }
            };
        }

        self.inner.extended_data(channel, ext, data, session).await
    }

    async fn xon_xoff(
        &mut self,
        channel: ChannelId,
        client_can_do: bool,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.xon_xoff(channel, client_can_do, session).await
    }

    async fn exit_status(
        &mut self,
        channel: ChannelId,
        exit_status: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.exit_status(channel, exit_status, session).await
    }

    #[allow(unused_variables)]
    async fn exit_signal(
        &mut self,
        channel: ChannelId,
        signal_name: Sig,
        core_dumped: bool,
        error_message: &str,
        lang_tag: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner
            .exit_signal(channel, signal_name, core_dumped, error_message, lang_tag, session)
            .await
    }

    async fn window_adjusted(
        &mut self,
        channel: ChannelId,
        new_size: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.window_adjusted(channel, new_size, session).await
    }

    fn adjust_window(&mut self, channel: ChannelId, window: u32) -> u32 {
        self.inner.adjust_window(channel, window)
    }

    async fn openssh_ext_host_keys_announced(
        &mut self,
        keys: Vec<PublicKey>,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.inner.openssh_ext_host_keys_announced(keys, session).await
    }

    async fn disconnected(&mut self, reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        self.inner.disconnected(reason).await
    }
}
