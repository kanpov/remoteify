use std::{
    collections::HashMap,
    sync::{atomic::AtomicU16, Arc},
};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use russh::{
    client::{self, DisconnectReason, Session},
    ChannelId, Sig,
};
use russh_keys::key;
use tokio::sync::RwLock;

use crate::terminal::{LinuxTerminalEvent, LinuxTerminalEventReceiver};

#[async_trait]
pub trait RusshGlobalReceiver: Send {
    #[allow(unused_variables)]
    async fn check_server_key(&mut self, server_public_key: &key::PublicKey) -> Result<bool, russh::Error> {
        Ok(true)
    }
}

// shared reference to the DHS
pub(super) static DHS: Lazy<DelegatingHandlerStorage> = Lazy::new(|| DelegatingHandlerStorage {
    hash_map: HashMap::new(),
});
// generator of IDs for the DHS
pub(super) static DHS_ID_GEN: AtomicU16 = AtomicU16::new(0);

// the DHS, mapping each RusshLinux to an rwlocked map of channel-ids to event receivers for those channels
pub(super) struct DelegatingHandlerStorage {
    pub hash_map: HashMap<u16, Arc<RwLock<HashMap<ChannelId, Box<dyn LinuxTerminalEventReceiver>>>>>,
}

pub(super) struct DelegatingHandler<R>
where
    R: RusshGlobalReceiver,
{
    pub dhs_id: u16,
    pub global_receiver: R,
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
        let dhs_ref = DHS
            .hash_map
            .get(&self.dhs_id)
            .ok_or(russh::Error::WrongChannel)?
            .read()
            .await;
        for receiver in dhs_ref.values() {
            receiver.receive_event(LinuxTerminalEvent::TerminalDisconnected);
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
    let dhs_ref = DHS
        .hash_map
        .get(&delegating_handler.dhs_id)
        .ok_or(russh::Error::WrongChannel)?
        .read()
        .await;

    if let Some(receiver) = dhs_ref.get(&channel) {
        receiver.receive_event(terminal_event).await;
    }
    Ok(())
}
