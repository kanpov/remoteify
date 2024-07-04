mod filesystem;
mod connection;

pub use connection::*;
use russh::client;

pub struct SshLinux<T>
where
    T: client::Handler,
    T: 'static,
{
    handle: client::Handle<T>,
}

#[cfg(test)]
mod tests {
    use russh::client;

    use crate::{
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
        let ssh_linux = SshLinux::connect(TrustingHandler {}, conn_opt).await;
        ssh_linux.unwrap();
    }
}
