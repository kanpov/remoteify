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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        let mut reader_writer = ssh_linux
            .file_open_read_write(Path::new("/tmp/a.txt"), false)
            .await
            .unwrap();
        let mut buf = String::new();
        reader_writer.read_to_string(&mut buf).await.unwrap();
        drop(reader_writer);
        dbg!(buf);

        ssh_linux.copy(Path::new("/tmp/c.txt"), Path::new("/tmp/d.txt")).await.unwrap();
    }
}
