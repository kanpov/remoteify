mod filesystem;
mod network;

use std::sync::Arc;

use openssh::Session;
use openssh_sftp_client::{Sftp, SftpOptions};
use tokio::sync::Mutex;

pub struct OpensshLinux {
    session: Arc<Session>,
    sftp_mutex: Arc<Mutex<Sftp>>,
}

#[derive(Debug)]
pub enum OpensshConnectionError {
    CheckError(openssh::Error),
    SftpEstablishError(openssh_sftp_client::Error),
}

impl OpensshLinux {
    pub async fn new(
        session: Session,
        sftp_session: Session,
        sftp_options: SftpOptions,
    ) -> Result<OpensshLinux, OpensshConnectionError> {
        session.check().await.map_err(OpensshConnectionError::CheckError)?;
        let sftp = Sftp::from_session(sftp_session, sftp_options)
            .await
            .map_err(OpensshConnectionError::SftpEstablishError)?;

        Ok(OpensshLinux {
            session: Arc::new(session),
            sftp_mutex: Arc::new(Mutex::new(sftp)),
        })
    }
}
