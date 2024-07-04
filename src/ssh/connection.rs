use std::sync::Arc;

use async_trait::async_trait;
use russh::client::{self};
use russh_keys::key::{KeyPair, PublicKey};

use crate::SshLinux;

#[derive(Debug)]
pub enum SshConnectionError<T>
where
    T: client::Handler,
    T: 'static,
{
    AuthenticationFailed,
    HandlerFailure(T::Error),
}

pub struct SshConnectionOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub config: client::Config,
    pub authentication: SshAuthentication,
}

pub enum SshAuthentication {
    Password { password: String },
    PublicKey { key_pair: KeyPair },
    None,
}

impl<T> SshLinux<T>
where
    T: client::Handler,
{
    pub async fn connect(
        handler: T,
        connection_options: SshConnectionOptions,
    ) -> Result<SshLinux<T>, SshConnectionError<T>> {
        let mut handle = match client::connect(
            Arc::new(connection_options.config),
            (connection_options.host, connection_options.port),
            handler,
        )
        .await
        {
            Ok(handle) => handle,
            Err(err) => return Err(SshConnectionError::HandlerFailure(err)),
        };

        match connection_options.authentication {
            SshAuthentication::Password { password } => {
                let error = map_to_error(
                    handle
                        .authenticate_password(connection_options.username, password)
                        .await,
                );
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
            SshAuthentication::PublicKey { key_pair } => {
                let error = map_to_error(
                    handle
                        .authenticate_publickey(connection_options.username, Arc::new(key_pair))
                        .await,
                );
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
            SshAuthentication::None => {
                let error =
                    map_to_error(handle.authenticate_none(connection_options.username).await);
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
        }

        Ok(SshLinux { handle })
    }
}

fn map_to_error<T>(result: Result<bool, russh::Error>) -> Option<SshConnectionError<T>>
where
    T: client::Handler,
{
    if result.is_err() {
        return Some(SshConnectionError::HandlerFailure(
            result.unwrap_err().into(),
        ));
    }
    if !result.unwrap() {
        return Some(SshConnectionError::AuthenticationFailed);
    }
    None
}

#[derive(Debug)]
pub struct TrustingHandler {}

#[async_trait]
impl client::Handler for TrustingHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}