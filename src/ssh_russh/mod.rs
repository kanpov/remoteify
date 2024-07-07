pub mod connection;
mod filesystem;
mod network;

use std::sync::Arc;

use russh::{
    client::{self, Msg},
    Channel,
};
use tokio::sync::Mutex;

pub struct RusshLinux<H>
where
    H: client::Handler,
{
    handle_mutex: Arc<Mutex<client::Handle<H>>>,
    fs_channel_mutex: Arc<Mutex<Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}
