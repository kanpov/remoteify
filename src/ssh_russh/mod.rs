pub mod connection;
pub mod event_receiver;
mod filesystem;
mod network;

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
    handle: Arc<Mutex<client::Handle<DelegatingHandler<R>>>>,
    fs_ssh_channel: Arc<Mutex<Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}
