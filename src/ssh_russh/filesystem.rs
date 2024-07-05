use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use russh::{client, ChannelMsg};
use russh_sftp::{
    client::fs::{File, Metadata},
    protocol::{FileType, OpenFlags},
};
use std::io::{self};

use crate::filesystem::{LinuxDirEntry, LinuxDirEntryType, LinuxFilesystem, LinuxOpenOptions};

use super::RusshLinux;

#[async_trait]
impl<T> LinuxFilesystem for RusshLinux<T>
where
    T: client::Handler,
{
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        internal_wrap_res(self.sftp_session.try_exists(conv_path(path)).await)
    }

    async fn open_file(&self, path: &Path, open_options: &LinuxOpenOptions) -> io::Result<File> {
        let mut flags = OpenFlags::empty();
        if open_options.is_read() {
            flags.insert(OpenFlags::READ);
        }
        if open_options.is_write() {
            flags.insert(OpenFlags::WRITE);
        }
        if open_options.is_append() {
            flags.insert(OpenFlags::APPEND);
        }
        if open_options.is_truncate() {
            flags.insert(OpenFlags::TRUNCATE);
        }
        if open_options.is_create() {
            flags.insert(OpenFlags::CREATE);
        }

        internal_wrap_res(self.sftp_session.open_with_flags(conv_path(path), flags).await)
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match self.sftp_session.create(conv_path(path)).await {
            Ok(file) => file,
            Err(err) => return Err(io::Error::other(err)),
        };
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        internal_wrap_res(self.sftp_session.rename(conv_path(old_path), conv_path(new_path)).await)
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        let result = internal_run_fs_command(self, format!("cp {} {}", conv_path(old_path), conv_path(new_path))).await;
        match result {
            Ok(_) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let path_buf = match self.sftp_session.canonicalize(conv_path(path)).await {
            Ok(path) => PathBuf::from(path),
            Err(err) => return Err(io::Error::other(err)),
        };
        Ok(path_buf)
    }

    async fn symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        internal_wrap_res(
            self.sftp_session
                .symlink(conv_path(source_path), conv_path(destination_path))
                .await,
        )
    }

    async fn hardlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        match internal_run_fs_command(
            self,
            format!("ln {} {}", conv_path(source_path), conv_path(destination_path)),
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        match self.sftp_session.read_link(conv_path(link_path)).await {
            Ok(string) => Ok(PathBuf::from(string)),
            Err(err) => Err(io::Error::other(err)),
        }
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
        internal_wrap_res(
            self.sftp_session
                .set_metadata(
                    conv_path(path),
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

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        internal_wrap_res(self.sftp_session.remove_file(conv_path(path)).await)
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        internal_wrap_res(self.sftp_session.create_dir(conv_path(path)).await)
    }

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()> {
        let mut current_path = String::new();
        let mut previous_existed = false;

        for component in path.components() {
            match component {
                std::path::Component::Normal(os_component_value) => {
                    let component_value = match os_component_value.to_str() {
                        Some(val) => val,
                        None => return Err(io::Error::other("could not convert os_str to str")),
                    };

                    current_path.push_str("/");
                    current_path.push_str(component_value);

                    if !previous_existed {
                        let exists = match self.sftp_session.try_exists(&current_path).await {
                            Ok(exists) => exists,
                            Err(err) => return Err(io::Error::other(err)),
                        };

                        if exists {
                            previous_existed = true;
                        }

                        continue;
                    }

                    match self.sftp_session.create_dir(&current_path).await {
                        Ok(_) => {}
                        Err(err) => return Err(io::Error::other(err)),
                    };
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>> {
        let read_dir = match self.sftp_session.read_dir(conv_path(path)).await {
            Ok(res) => res,
            Err(err) => return Err(io::Error::other(err)),
        };
        let entries = read_dir
            .map(|dir_entry| {
                LinuxDirEntry::new(
                    dir_entry.file_name(),
                    dir_entry.file_type().into(),
                    path.join(Path::new(&dir_entry.file_name())),
                )
            })
            .collect::<Vec<_>>();

        Ok(entries)
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        internal_wrap_res(self.sftp_session.remove_dir(conv_path(path)).await)
    }
}

impl From<FileType> for LinuxDirEntryType {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Dir => LinuxDirEntryType::Dir,
            FileType::File => LinuxDirEntryType::File,
            FileType::Symlink => LinuxDirEntryType::Symlink,
            FileType::Other => LinuxDirEntryType::Other,
        }
    }
}

async fn internal_run_fs_command<T>(instance: &RusshLinux<T>, command: String) -> io::Result<Option<u32>>
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
    match result {
        Ok(val) => Ok(val),
        Err(err) => Err(io::Error::other(err)),
    }
}

fn conv_path(path: &Path) -> String {
    String::from(path.to_str().unwrap())
}
