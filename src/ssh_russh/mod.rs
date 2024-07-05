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

#[cfg(test)]
mod tests {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt, path::Path};

    use russh::client;

    use crate::{
        filesystem::LinuxFilesystem,
        ssh_russh::connection::{RusshAuthentication, RusshConnectionOptions, TrustingHandler},
        ssh_russh::RusshLinux,
    };

    #[tokio::test]
    async fn tmp_test() {
        let conn_opt = RusshConnectionOptions {
            host: "localhost".into(),
            port: 9000,
            config: client::Config::default(),
            username: "root".into(),
            authentication: RusshAuthentication::Password {
                password: "root123".into(),
            },
        };
        let ssh_linux = RusshLinux::connect(TrustingHandler {}, conn_opt)
            .await
            .unwrap();

        let loc = ssh_linux
            .read_link(Path::new("/tmp/link.txt"))
            .await
            .unwrap();
        dbg!(loc);

        ssh_linux
            .set_permissions(Path::new("/tmp/b.txt"), Permissions::from_mode(1000))
            .await
            .unwrap();
    }
}
