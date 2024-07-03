use std::sync::Arc;

use async_trait::async_trait;
use russh::client::{self, Handle};
use russh_keys::key::{KeyPair, PublicKey};

mod filesystem;

pub struct SshLinux<T>
where
    T: client::Handler,
    T: 'static,
{
    handle: Handle<T>,
}

pub struct SshConnectionOptions {
    pub host: String,
    pub port: u16,
    pub config: client::Config,
    pub authentication_method: SshAuthenticationMethod,
}

pub enum SshAuthenticationMethod {
    Password { username: String, password: String },
    PublicKey { username: String, key_pair: KeyPair },
    None { username: String }
}

impl<T> SshLinux<T>
where
    T: client::Handler,
{
    pub async fn connect(
        handler: T,
        connection_options: SshConnectionOptions,
    ) -> Result<SshLinux<T>, T::Error> {
        let mut handle = 
            client::connect(
                Arc::new(connection_options.config),
                (connection_options.host, connection_options.port),
                handler,
            )
            .await?;

        match connection_options.authentication_method {
            SshAuthenticationMethod::Password { username, password } => {
                handle
                    .authenticate_password(username, password)
                    .await?;
            }
            SshAuthenticationMethod::None { username } => {
                handle.authenticate_none(username).await?;
            },
            SshAuthenticationMethod::PublicKey { username, key_pair } => {
                handle.authenticate_publickey(username, Arc::new(key_pair)).await?;
            },
        }

        Ok(SshLinux { handle })
    }
}

pub struct TrustingHandler {}

#[async_trait]
impl client::Handler for TrustingHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use russh::client;

    use crate::{ssh::{SshAuthenticationMethod, SshConnectionOptions, TrustingHandler}, SshLinux};

    #[tokio::test]
    async fn tmp_test() {
        let conn_opt = SshConnectionOptions {
            host: "localhost".into(),
            port: 9000,
            config: client::Config::default(),
            authentication_method: SshAuthenticationMethod::Password {
                username: "root".into(),
                password: "root123".into()
            }
        };
        let ssh_linux = SshLinux::connect(TrustingHandler {}, conn_opt).await.unwrap();
    }
}
