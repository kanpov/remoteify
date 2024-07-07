use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use russh::ChannelMsg;
use russh_sftp::{
    client::fs::{File, Metadata},
    protocol::{FileAttributes, FileType, OpenFlags},
};
use std::io::{self};

use crate::filesystem::{LinuxDirEntry, LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions};

use super::{event_receiver::RusshGlobalReceiver, RusshLinux};

#[async_trait]
impl<R> LinuxFilesystem for RusshLinux<R>
where
    R: RusshGlobalReceiver,
{
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        self.sftp_session
            .try_exists(conv_path(path))
            .await
            .map_err(io::Error::other)
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

        self.sftp_session
            .open_with_flags(conv_path(path), flags)
            .await
            .map_err(io::Error::other)
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = self
            .sftp_session
            .create(conv_path(path))
            .await
            .map_err(io::Error::other)?;
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        self.sftp_session
            .rename(conv_path(old_path), conv_path(new_path))
            .await
            .map_err(io::Error::other)
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        run_fs_command(self, format!("cp {} {}", conv_path(old_path), conv_path(new_path)))
            .await
            .map(|_| None)
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.sftp_session
            .canonicalize(conv_path(path))
            .await
            .map(PathBuf::from)
            .map_err(io::Error::other)
    }

    async fn create_symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        self.sftp_session
            .symlink(conv_path(source_path), conv_path(destination_path))
            .await
            .map_err(io::Error::other)
    }

    async fn create_hard_link(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        run_fs_command(
            self,
            format!("ln {} {}", conv_path(source_path), conv_path(destination_path)),
        )
        .await
        .map(|_| ())
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        self.sftp_session
            .read_link(conv_path(link_path))
            .await
            .map(PathBuf::from)
            .map_err(io::Error::other)
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
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
            .await
            .map_err(io::Error::other)
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.sftp_session
            .remove_file(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.sftp_session
            .create_dir(conv_path(path))
            .await
            .map_err(io::Error::other)
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
        let read_dir = self
            .sftp_session
            .read_dir(conv_path(path))
            .await
            .map_err(io::Error::other)?;
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
        self.sftp_session
            .remove_dir(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn remove_dir_recursively(&self, path: &Path) -> io::Result<()> {
        let str_path = path
            .to_str()
            .ok_or(io::Error::other("could not convert &Path to str"))?;

        run_fs_command(self, format!("rm -r {}", str_path)).await.map(|_| ())
    }

    async fn get_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        self.sftp_session
            .metadata(conv_path(path))
            .await
            .map(|attrs| attrs.into())
            .map_err(io::Error::other)
    }

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        self.sftp_session
            .symlink_metadata(conv_path(path))
            .await
            .map(|attrs| attrs.into())
            .map_err(io::Error::other)
    }
}

impl From<FileType> for LinuxFileType {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Dir => LinuxFileType::Dir,
            FileType::File => LinuxFileType::File,
            FileType::Symlink => LinuxFileType::Symlink,
            FileType::Other => LinuxFileType::Other,
        }
    }
}

async fn run_fs_command<T>(instance: &RusshLinux<T>, command: String) -> io::Result<Option<u32>>
where
    T: RusshGlobalReceiver,
{
    let mut chan = instance.fs_channel_mutex.lock().await;
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

fn conv_path(path: &Path) -> String {
    String::from(path.to_str().unwrap())
}

impl Into<LinuxFileMetadata> for FileAttributes {
    fn into(self) -> LinuxFileMetadata {
        LinuxFileMetadata::new(
            Some(self.file_type().into()),
            self.size,
            match self.permissions {
                Some(bit) => Some(Permissions::from_mode(bit)),
                None => None,
            },
            self.modified().ok(),
            self.accessed().ok(),
            None,
            self.uid,
            self.user,
            self.gid,
            self.group,
        )
    }
}
