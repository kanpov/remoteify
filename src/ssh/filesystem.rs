use std::path::Path;

use async_trait::async_trait;
use russh::client;
use russh_sftp::protocol::OpenFlags;
use tokio::io::{self, AsyncWriteExt};

use crate::filesystem::LinuxFilesystem;

use super::SshLinux;

#[async_trait]
impl<T> LinuxFilesystem for SshLinux<T>
where
    T: client::Handler,
{
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        wrap_res(
            self.sftp_session
                .as_ref()
                .try_exists(path_to_str(path))
                .await,
        )
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match self.sftp_session.as_ref().create(path_to_str(path)).await {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        drop(file);
        Ok(())
    }

    async fn write_text_to_file(&self, path: &Path, text: &String) -> io::Result<()> {
        self.write_bytes_to_file(path, text.as_bytes()).await
    }

    async fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = match self
            .sftp_session
            .open_with_flags(path_to_str(path), OpenFlags::WRITE)
            .await
        {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        file.write_all(bytes).await
    }

    async fn append_text_to_file(&self, path: &Path, text: &String) -> io::Result<()> {
        self.append_bytes_to_file(path, text.as_bytes()).await
    }

    async fn append_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = match self
            .sftp_session
            .open_with_flags(
                path_to_str(path),
                OpenFlags::union(OpenFlags::WRITE, OpenFlags::APPEND),
            )
            .await
        {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        file.write_all(bytes).await
    }
}

fn wrap_res<T>(result: Result<T, russh_sftp::client::error::Error>) -> io::Result<T> {
    if result.is_err() {
        return Err(io::Error::other(result.err().unwrap()));
    }
    Ok(result.unwrap())
}

fn path_to_str(path: &Path) -> String {
    String::from(path.to_str().unwrap())
}
