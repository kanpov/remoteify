use std::path::Path;

use async_trait::async_trait;
use russh::client;
use russh_sftp::{client::fs::File, protocol::OpenFlags};
use tokio::io::{self};

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

    async fn get_file_writer(&self, path: &Path) -> io::Result<File> {
        match self
            .sftp_session
            .open_with_flags(path_to_str(path), OpenFlags::WRITE)
            .await
        {
            Ok(file) => return Ok(file),
            Err(err) => return Err(io::Error::other(err)),
        }
    }

    async fn get_file_writer_for_append(&self, path: &Path) -> io::Result<File> {
        match self
            .sftp_session
            .open_with_flags(path_to_str(path), OpenFlags::union(OpenFlags::WRITE, OpenFlags::APPEND))
            .await
        {
            Ok(file) => return Ok(file),
            Err(err) => return Err(io::Error::other(err)),
        }
    }

    async fn get_file_reader(&self, path: &Path) -> io::Result<File> {
        match self
            .sftp_session
            .open_with_flags(path_to_str(path), OpenFlags::READ)
            .await
        {
            Ok(file) => return Ok(file),
            Err(err) => return Err(io::Error::other(err)),
        }
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match self.sftp_session.as_ref().create(path_to_str(path)).await {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        drop(file);
        Ok(())
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
