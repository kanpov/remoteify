use std::sync::Arc;

use async_trait::async_trait;
use russh::client::{self};
use russh_keys::key::{KeyPair, PublicKey};
use russh_sftp::client::SftpSession;

use crate::SshLinux;

#[derive(Debug)]
pub enum SshConnectionError<T>
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
            Err(err) => return Err(SshConnectionError::ConnectionError(err)),
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

        let sftp_channel_result = handle.channel_open_session().await;
        if sftp_channel_result.is_err() {
            return Err(SshConnectionError::ChannelOpenError(
                sftp_channel_result.unwrap_err(),
            ));
        }
        let sftp_channel = sftp_channel_result.unwrap();

        let sftp_result = sftp_channel.request_subsystem(true, "sftp").await;
        if sftp_result.is_err() {
            return Err(SshConnectionError::SftpRequestError(
                sftp_result.unwrap_err(),
            ));
        }

        let sftp_session_result = SftpSession::new(sftp_channel.into_stream()).await;
        if sftp_session_result.is_err() {
            return Err(SshConnectionError::SftpOpenError(
                sftp_session_result.err().unwrap(),
            ));
        }
        let sftp_session = sftp_session_result.unwrap();

        let ssh_channel_result = handle.channel_open_session().await;
        if ssh_channel_result.is_err() {
            return Err(SshConnectionError::ChannelOpenError(
                ssh_channel_result.unwrap_err(),
            ));
        }

        Ok(SshLinux {
            handle: Arc::new(handle),
            ssh_channel: Arc::new(ssh_channel_result.unwrap()),
            sftp_session: Arc::new(sftp_session),
        })
    }
}

fn map_to_error<T>(result: Result<bool, russh::Error>) -> Option<SshConnectionError<T>>
where
    T: client::Handler,
{
    if result.is_err() {
        return Some(SshConnectionError::AuthenticationError(result.unwrap_err()));
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
