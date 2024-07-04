use std::sync::Arc;

use async_trait::async_trait;
use russh::client::{self};
use russh_keys::key::{KeyPair, PublicKey};
use russh_sftp::client::SftpSession;
use tokio::sync::Mutex;

use crate::ssh_russh::RusshLinux;

#[derive(Debug)]
pub enum RusshConnectionError<T>
where
    T: client::Handler,
    T: 'static,
{
    ConnectionError(T::Error),
    AuthenticationError(russh::Error),
    ChannelOpenError(russh::Error),
    SftpRequestError(russh::Error),
    SftpOpenError(russh_sftp::client::error::Error),
}

pub struct RusshConnectionOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub config: client::Config,
    pub authentication: RusshAuthentication,
}

pub enum RusshAuthentication {
    Password { password: String },
    PublicKey { key_pair: KeyPair },
    None,
}

impl<T> RusshLinux<T>
where
    T: client::Handler,
{
    pub async fn connect(
        handler: T,
        connection_options: RusshConnectionOptions,
    ) -> Result<RusshLinux<T>, RusshConnectionError<T>> {
        let mut handle = match client::connect(
            Arc::new(connection_options.config),
            (connection_options.host, connection_options.port),
            handler,
        )
        .await
        {
            Ok(handle) => handle,
            Err(err) => return Err(RusshConnectionError::ConnectionError(err)),
        };

        match connection_options.authentication {
            RusshAuthentication::Password { password } => {
                let error = map_to_error(
                    handle
                        .authenticate_password(connection_options.username, password)
                        .await,
                );
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
            RusshAuthentication::PublicKey { key_pair } => {
                let error = map_to_error(
                    handle
                        .authenticate_publickey(connection_options.username, Arc::new(key_pair))
                        .await,
                );
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
            RusshAuthentication::None => {
                let error =
                    map_to_error(handle.authenticate_none(connection_options.username).await);
                if error.is_some() {
                    return Err(error.unwrap());
                }
            }
        }

        let sftp_channel = match handle.channel_open_session().await {
            Ok(channel) => channel,
            Err(err) => return Err(RusshConnectionError::ChannelOpenError(err)),
        };

        match sftp_channel.request_subsystem(true, "sftp").await {
            Ok(_) => {}
            Err(err) => return Err(RusshConnectionError::SftpRequestError(err)),
        }

        let sftp_session = match SftpSession::new(sftp_channel.into_stream()).await {
            Ok(session) => session,
            Err(err) => return Err(RusshConnectionError::SftpOpenError(err)),
        };

        let ssh_channel = match handle.channel_open_session().await {
            Ok(channel) => channel,
            Err(err) => return Err(RusshConnectionError::ChannelOpenError(err)),
        };

        Ok(RusshLinux {
            handle: Arc::new(handle),
            ssh_channel: Arc::new(Mutex::new(ssh_channel)),
            sftp_session: Arc::new(sftp_session),
        })
    }
}

fn map_to_error<T>(result: Result<bool, russh::Error>) -> Option<RusshConnectionError<T>>
where
    T: client::Handler,
{
    if result.is_err() {
        return Some(RusshConnectionError::AuthenticationError(
            result.unwrap_err(),
        ));
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
