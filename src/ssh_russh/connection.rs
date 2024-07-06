use std::{collections::HashMap, sync::Arc};

use russh::client::{self};
use russh_keys::key::KeyPair;
use russh_sftp::client::SftpSession;
use tokio::sync::Mutex;

use crate::ssh_russh::RusshLinux;

use super::event_receiver::{DelegatingHandler, RusshGlobalReceiver};

#[derive(Debug)]
pub enum RusshConnectionError {
    ConnectionError(russh::Error),
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

impl<R> RusshLinux<R>
where
    R: RusshGlobalReceiver,
{
    pub async fn connect(
        event_receiver: R,
        connection_options: RusshConnectionOptions,
    ) -> Result<RusshLinux<R>, RusshConnectionError>
    where
        R: 'static,
    {
        let mut handle = match client::connect(
            Arc::new(connection_options.config),
            (connection_options.host, connection_options.port),
            DelegatingHandler {
                global_receiver: event_receiver,
                terminal_receivers: HashMap::new(),
            },
        )
        .await
        {
            Ok(handle) => handle,
            Err(err) => return Err(RusshConnectionError::ConnectionError(err)),
        };

        match connection_options.authentication {
            RusshAuthentication::Password { password } => map_to_error::<R>(
                handle
                    .authenticate_password(connection_options.username, password)
                    .await,
            )?,
            RusshAuthentication::PublicKey { key_pair } => map_to_error::<R>(
                handle
                    .authenticate_publickey(connection_options.username, Arc::new(key_pair))
                    .await,
            )?,
            RusshAuthentication::None => {
                map_to_error::<R>(handle.authenticate_none(connection_options.username).await)?
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
            handle: Arc::new(Mutex::new(handle)),
            ssh_channel: Arc::new(Mutex::new(ssh_channel)),
            sftp_session: Arc::new(sftp_session),
        })
    }
}

fn map_to_error<T>(result: Result<bool, russh::Error>) -> Result<(), RusshConnectionError>
where
    T: RusshGlobalReceiver,
{
    result
        .map(|_| ())
        .map_err(|err| RusshConnectionError::AuthenticationError(err))
}
