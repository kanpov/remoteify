use std::{fmt::Debug, path::Path};

use async_trait::async_trait;
use russh::client;
use tokio::io;

use crate::filesystem::LinuxFilesystem;

use super::SshLinux;

#[async_trait]
impl<T> LinuxFilesystem for SshLinux<T>
where
    T: client::Handler,
{
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        wrap_res(self.sftp_session.as_ref().try_exists(path_to_str(path)).await)
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file_result = self.sftp_session.as_ref().create(path_to_str(path)).await;
        if file_result.is_err() {
            return Err(io::Error::other(file_result.err().unwrap()))
        }
        let file = file_result.unwrap();
        drop(file);
        Ok(())
    }

    async fn write_text_to_file(&self, path: &Path, text: &String) -> io::Result<()> {
        todo!()
    }

    async fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        todo!()
    }
}

fn wrap_res<T>(result: Result<T, russh_sftp::client::error::Error>) -> io::Result<T> {
    if result.is_err() {
        return Err(io::Error::other(result.err().unwrap()))
    }
    Ok(result.unwrap())
}

fn path_to_str(path: &Path) -> String {
    String::from(path.to_str().unwrap())
}
