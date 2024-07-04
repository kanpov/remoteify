mod connection;
mod filesystem;

use std::sync::Arc;

pub use connection::*;
use russh::client::{self, Msg};

pub struct SshLinux<T>
where
    T: client::Handler,
    T: 'static,
{
    handle: Arc<client::Handle<T>>,
    ssh_channel: Arc<russh::Channel<Msg>>,
    sftp_session: Arc<russh_sftp::client::SftpSession>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use russh::client;

    use crate::{
        filesystem::LinuxFilesystem,
        ssh::{SshAuthentication, SshConnectionOptions, TrustingHandler},
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
        ssh_linux
            .write_text_to_file(Path::new("/tmp/a.txt"), &"testa".to_string())
            .await
            .unwrap();
        dbg!(true);
    }
}
