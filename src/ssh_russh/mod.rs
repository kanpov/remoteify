pub mod connection;
mod filesystem;

use std::sync::Arc;

use russh::client::{self, Msg};
use tokio::sync::Mutex;

pub struct RusshLinux<T>
where
    T: client::Handler,
    T: 'static,
{
    _handle: Arc<client::Handle<T>>,
    ssh_channel: Arc<Mutex<russh::Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}
