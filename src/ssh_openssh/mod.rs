mod filesystem;

use std::sync::Arc;

use openssh::{Session, Stdio};
use openssh_sftp_client::{Sftp, SftpOptions};
use tokio::sync::Mutex;

pub struct OpensshLinux {
    session: Arc<Session>,
    sftp_mutex: Arc<Mutex<Sftp>>,
}

pub enum OpensshConnectionError {
    CheckError(openssh::Error),
    SftpRequestError(openssh::Error),
    SftpStdinNotFound,
    SftpStdoutNotFound,
    SftpEstablishError(openssh_sftp_client::Error),
}

impl OpensshLinux {
    async fn new(session: Session, sftp_options: SftpOptions) -> Result<OpensshLinux, OpensshConnectionError> {
        session.check().await.map_err(OpensshConnectionError::CheckError)?;
        let mut child = session
            .subsystem("sftp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .await
            .map_err(OpensshConnectionError::SftpRequestError)?;

        let sftp = Sftp::new(
            child.stdin().take().ok_or(OpensshConnectionError::SftpStdinNotFound)?,
            child
                .stdout()
                .take()
                .ok_or(OpensshConnectionError::SftpStdoutNotFound)?,
            sftp_options,
        )
        .await
        .map_err(OpensshConnectionError::SftpEstablishError)?;

        Ok(OpensshLinux {
            session: Arc::new(session),
            sftp_mutex: Arc::new(Mutex::new(sftp)),
        })
    }
}
