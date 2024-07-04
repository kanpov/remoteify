pub mod connection;
mod filesystem;

use std::sync::Arc;

use russh::client::{self, Msg};
use tokio::sync::Mutex;

pub struct SshLinux<T>
where
    T: client::Handler,
    T: 'static,
{
    handle: Arc<client::Handle<T>>,
    ssh_channel: Arc<Mutex<russh::Channel<Msg>>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use russh::client;
    use tokio::io::AsyncReadExt;

    use crate::{
        filesystem::LinuxFilesystem,
        ssh::connection::{SshAuthentication, SshConnectionOptions, TrustingHandler},
        SshLinux,
    };

    #[tokio::test]
    async fn tmp_test() {
        let conn_opt = SshConnectionOptions {
            host: "localhost".into(),
            port: 9000,
            config: client::Config::default(),
            username: "root".into(),
            authentication: SshAuthentication::Password {
                password: "root123".into(),
            },
        };
        let ssh_linux = SshLinux::connect(TrustingHandler {}, conn_opt)
            .await
            .unwrap();
        let path_buf = ssh_linux.canonicalize(Path::new("/tmp/..")).await.unwrap();
        dbg!(path_buf);
    }
}
