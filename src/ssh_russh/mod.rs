pub mod connection;
pub mod event_receiver;
mod filesystem;
mod network;
mod terminal;

use std::sync::Arc;

use event_receiver::{DelegatingHandler, RusshGlobalReceiver};
use russh::{
    client::{self, Msg},
    Channel,
};
use tokio::sync::Mutex;

pub struct RusshLinux<R>
where
    R: RusshGlobalReceiver,
{
    handle_mutex: Arc<Mutex<client::Handle<DelegatingHandler<R>>>>,
    fs_channel_mutex: Arc<Mutex<Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}
