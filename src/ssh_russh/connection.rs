use std::sync::Arc;

use russh::client;
use russh_keys::key::KeyPair;
use russh_sftp::client::SftpSession;
use tokio::sync::Mutex;

use crate::ssh_russh::RusshLinux;

#[derive(Debug)]
pub enum RusshConnectionError<H>
where
    H: client::Handler,
{
    ConnectionError(H::Error),
    AuthenticationError(russh::Error),
    SftpChannelOpenError(russh::Error),
    SshChannelOpenError(russh::Error),
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

impl<H> RusshLinux<H>
where
    H: client::Handler,
{
    pub async fn connect(
        handler: H,
        connection_options: RusshConnectionOptions,
    ) -> Result<RusshLinux<H>, RusshConnectionError<H>>
    where
        H: 'static,
    {
        let mut handle = client::connect(
            Arc::new(connection_options.config),
            (connection_options.host, connection_options.port),
            handler,
        )
        .await
        .map_err(|err| RusshConnectionError::ConnectionError(err))?;

        match connection_options.authentication {
            RusshAuthentication::Password { password } => map_to_error::<H>(
                handle
                    .authenticate_password(connection_options.username, password)
                    .await,
            )?,
            RusshAuthentication::PublicKey { key_pair } => map_to_error::<H>(
                handle
                    .authenticate_publickey(connection_options.username, Arc::new(key_pair))
                    .await,
            )?,
            RusshAuthentication::None => {
                map_to_error::<H>(handle.authenticate_none(connection_options.username).await)?
            }
        }

        let sftp_channel = handle
            .channel_open_session()
            .await
            .map_err(|err| RusshConnectionError::SshChannelOpenError(err))?;

        sftp_channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|err| RusshConnectionError::SftpRequestError(err))?;

        let sftp_session = SftpSession::new(sftp_channel.into_stream())
            .await
            .map_err(|err| RusshConnectionError::SftpOpenError(err))?;

        let fs_ssh_channel = handle
            .channel_open_session()
            .await
            .map_err(|err| RusshConnectionError::SftpChannelOpenError(err))?;

        Ok(RusshLinux {
            handle_mutex: Arc::new(Mutex::new(handle)),
            fs_channel_mutex: Arc::new(Mutex::new(fs_ssh_channel)),
            sftp_session: Arc::new(sftp_session),
        })
    }
}

fn map_to_error<H>(result: Result<bool, russh::Error>) -> Result<(), RusshConnectionError<H>>
where
    H: client::Handler,
{
    result
        .map(|_| ())
        .map_err(|err| RusshConnectionError::AuthenticationError(err))
}
