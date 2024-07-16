#[cfg(feature = "executor")]
mod executor;
#[cfg(feature = "filesystem")]
mod filesystem;
#[cfg(feature = "network")]
mod network;

use std::sync::Arc;

use openssh::Session;
use openssh_sftp_client::{Sftp, SftpOptions};
use tokio::sync::Mutex;

#[allow(unused)]
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
    pub async fn with_known_sftp(session: Session, sftp: Sftp) -> Result<OpensshLinux, OpensshConnectionError> {
        session.check().await.map_err(OpensshConnectionError::CheckError)?;
        Ok(OpensshLinux {
            session: Arc::new(session),
            sftp_mutex: Arc::new(Mutex::new(sftp)),
        })
    }

    pub async fn with_new_sftp(
        session: Session,
        sftp_options: SftpOptions,
    ) -> Result<OpensshLinux, OpensshConnectionError> {
        session.check().await.map_err(OpensshConnectionError::CheckError)?;
        let session_arc = Arc::new(session);
        let sftp = Sftp::from_clonable_session(session_arc.clone(), sftp_options)
            .await
            .map_err(OpensshConnectionError::SftpEstablishError)?;

        Ok(OpensshLinux {
            session: session_arc,
            sftp_mutex: Arc::new(Mutex::new(sftp)),
        })
    }
}
