use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use russh::{client, ChannelMsg};
use russh_sftp::{
    client::{
        fs::{File, Metadata},
        SftpSession,
    },
    protocol::OpenFlags,
};
use std::io::{self};

use crate::filesystem::LinuxFilesystem;

use super::RusshLinux;

#[async_trait]
impl<T> LinuxFilesystem for RusshLinux<T>
where
    T: client::Handler,
{
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        internal_wrap_res(self.sftp_session.try_exists(path_to_str(path)).await)
    }

    async fn file_open_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(&self.sftp_session, path, OpenFlags::WRITE, truncate).await
    }

    async fn file_open_append(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(
            &self.sftp_session,
            path,
            OpenFlags::union(OpenFlags::WRITE, OpenFlags::APPEND),
            truncate,
        )
        .await
    }

    async fn file_open_read(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(&self.sftp_session, path, OpenFlags::READ, truncate).await
    }

    async fn file_open_read_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(
            &self.sftp_session,
            path,
            OpenFlags::union(OpenFlags::READ, OpenFlags::WRITE),
            truncate,
        )
        .await
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match self.sftp_session.create(path_to_str(path)).await {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        internal_wrap_res(
            self.sftp_session
                .rename(path_to_str(old_path), path_to_str(new_path))
                .await,
        )
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        let result = internal_run_fs_command(
            self,
            format!("cp {} {}", path_to_str(old_path), path_to_str(new_path)),
        )
        .await;
        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let path_buf = match self.sftp_session.canonicalize(path_to_str(path)).await {
            Ok(path) => PathBuf::from(path),
            Err(err) => return Err(io::Error::other(err)),
        };
        Ok(path_buf)
    }

    async fn symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        internal_wrap_res(
            self.sftp_session
                .symlink(path_to_str(source_path), path_to_str(destination_path))
                .await,
        )
    }

    async fn hardlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        match internal_run_fs_command(
            self,
            format!(
                "ln {} {}",
                path_to_str(source_path),
                path_to_str(destination_path)
            ),
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        match self.sftp_session.read_link(path_to_str(link_path)).await {
            Ok(string) => Ok(PathBuf::from(string)),
            Err(err) => Err(io::Error::other(err)),
        }
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
        internal_wrap_res(
            self.sftp_session
                .set_metadata(
                    path_to_str(path),
                    Metadata {
                        size: None,
                        uid: None,
                        user: None,
                        gid: None,
                        group: None,
                        permissions: Some(permissions.mode()),
                        atime: None,
                        mtime: None,
                    },
                )
                .await,
        )
    }
}

async fn internal_open_file(
    sftp_session: &Arc<SftpSession>,
    path: &Path,
    base_flags: OpenFlags,
    truncate: bool,
) -> io::Result<File> {
    let mut flags = base_flags;
    if truncate {
        flags = OpenFlags::union(flags, OpenFlags::TRUNCATE);
    }

    match sftp_session
        .open_with_flags(path_to_str(&path), flags)
        .await
    {
        Ok(file) => return Ok(file),
        Err(err) => return Err(io::Error::other(err)),
    }
}

async fn internal_run_fs_command<T>(
    instance: &RusshLinux<T>,
    command: String,
) -> io::Result<Option<u32>>
where
    T: client::Handler,
{
    let mut chan = instance.ssh_channel.lock().await;
    let exec_result = chan.exec(true, command).await;
    if exec_result.is_err() {
        return Err(io::Error::other(exec_result.unwrap_err()));
    }

    let mut code = None;

    loop {
        let Some(msg) = chan.wait().await else {
            break;
        };
        match msg {
            ChannelMsg::ExitStatus { exit_status } => {
                code = Some(exit_status);
            }
            _ => {}
        }
    }

    Ok(code)
}

fn internal_wrap_res<T>(result: Result<T, russh_sftp::client::error::Error>) -> io::Result<T> {
    if result.is_err() {
        return Err(io::Error::other(result.err().unwrap()));
    }
    Ok(result.unwrap())
}

fn path_to_str(path: &Path) -> String {
    String::from(path.to_str().unwrap())
}
