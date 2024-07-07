use std::collections::HashMap;

use async_trait::async_trait;
use russh::{
    client::{self, DisconnectReason, Session},
    ChannelId, Sig,
};
use russh_keys::key;

use crate::terminal::{LinuxTerminalEvent, LinuxTerminalEventReceiver};

#[async_trait]
pub trait RusshGlobalReceiver: Send {
    #[allow(unused_variables)]
    async fn check_server_key(&mut self, server_public_key: &key::PublicKey) -> Result<bool, russh::Error> {
        Ok(true)
    }
}

pub(super) struct DelegatingHandler<R>
where
    R: RusshGlobalReceiver,
{
    pub global_receiver: R,
    pub terminal_receivers: HashMap<ChannelId, Box<dyn LinuxTerminalEventReceiver>>,
}

#[async_trait]
impl<R> client::Handler for DelegatingHandler<R>
where
    R: RusshGlobalReceiver,
{
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_public_key: &key::PublicKey) -> Result<bool, Self::Error> {
        self.global_receiver.check_server_key(server_public_key).await
    }

    async fn channel_eof(&mut self, channel: ChannelId, _session: &mut Session) -> Result<(), Self::Error> {
        send_off(self, channel, LinuxTerminalEvent::EOFReceived).await
    }

    async fn data(&mut self, channel: ChannelId, data: &[u8], _session: &mut Session) -> Result<(), Self::Error> {
        send_off(self, channel, LinuxTerminalEvent::DataReceived { data }).await
    }

    async fn extended_data(
        &mut self,
        channel: ChannelId,
        ext: u32,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        send_off(
            self,
            channel,
            LinuxTerminalEvent::ExtendedDataReceived {
                ext,
                extended_data: data,
            },
        )
        .await
    }

    async fn xon_xoff(
        &mut self,
        channel: ChannelId,
        client_can_do: bool,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        send_off(
            self,
            channel,
            LinuxTerminalEvent::XonXoffAbilityReceived {
                can_perform_xon_xoff: client_can_do,
            },
        )
        .await
    }

    async fn exit_status(
        &mut self,
        channel: ChannelId,
        exit_status: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        send_off(self, channel, LinuxTerminalEvent::ProcessExitedNormally { exit_status }).await
    }

    async fn exit_signal(
        &mut self,
        channel: ChannelId,
        signal_name: Sig,
        core_dumped: bool,
        error_message: &str,
        lang_tag: &str,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        send_off(
            self,
            channel,
            LinuxTerminalEvent::ProcessExitedAfterSignal {
                signal: &conv_sig_to_str(signal_name),
                core_dumped,
                error_message,
                lang_tag,
            },
        )
        .await
    }

    async fn window_adjusted(
        &mut self,
        channel: ChannelId,
        new_size: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        send_off(self, channel, LinuxTerminalEvent::WindowAdjusted { new_size }).await
    }

    async fn disconnected(&mut self, _reason: DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        for receiver in self.terminal_receivers.values() {
            receiver.receive_event(LinuxTerminalEvent::TerminalDisconnected).await;
        }
        Ok(())
    }
}

fn conv_sig_to_str(sig: Sig) -> String {
    match sig {
        Sig::ABRT => "ABRT".into(),
        Sig::ALRM => "ALRM".into(),
        Sig::FPE => "FPE".into(),
        Sig::HUP => "HUP".into(),
        Sig::ILL => "ILL".into(),
        Sig::INT => "INT".into(),
        Sig::KILL => "KILL".into(),
        Sig::PIPE => "PIPE".into(),
        Sig::QUIT => "QUIT".into(),
        Sig::SEGV => "SEGV".into(),
        Sig::TERM => "TERM".into(),
        Sig::USR1 => "USR1".into(),
        Sig::Custom(value) => value,
    }
}

async fn send_off<'a, R>(
    delegating_handler: &mut DelegatingHandler<R>,
    channel: ChannelId,
    terminal_event: LinuxTerminalEvent<'a>,
) -> Result<(), russh::Error>
where
    R: RusshGlobalReceiver,
{
    if let Some(receiver) = delegating_handler.terminal_receivers.get(&channel) {
        receiver.receive_event(terminal_event).await;
    }
    Ok(())
}
