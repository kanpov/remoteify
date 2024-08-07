use std::{
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use async_trait::async_trait;
use russh::{client, ChannelMsg};
use russh_sftp::{
    client::fs::{File, Metadata},
    protocol::{FileAttributes, FileType, OpenFlags},
};
use std::io::{self};

use crate::filesystem::{
    LinuxDirEntry, LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions, LinuxPermissions,
};

use super::RusshLinux;

#[async_trait]
impl<H> LinuxFilesystem for RusshLinux<H>
where
    H: client::Handler,
{
    async fn exists(&self, path: &OsStr) -> io::Result<bool> {
        self.sftp_session
            .try_exists(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn open_file(&self, path: &OsStr, open_options: &LinuxOpenOptions) -> io::Result<File> {
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

    async fn create_file(&self, path: &OsStr) -> io::Result<()> {
        let file = self
            .sftp_session
            .create(conv_path(path))
            .await
            .map_err(io::Error::other)?;
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &OsStr, new_path: &OsStr) -> io::Result<()> {
        self.sftp_session
            .rename(conv_path(old_path), conv_path(new_path))
            .await
            .map_err(io::Error::other)
    }

    async fn copy_file(&self, old_path: &OsStr, new_path: &OsStr) -> io::Result<Option<u64>> {
        run_fs_command(self, format!("cp {} {}", conv_path(old_path), conv_path(new_path)))
            .await
            .map(|_| None)
    }

    async fn canonicalize(&self, path: &OsStr) -> io::Result<OsString> {
        self.sftp_session
            .canonicalize(conv_path(path))
            .await
            .map(|path| path.into())
            .map_err(io::Error::other)
    }

    async fn create_symlink(&self, source_path: &OsStr, destination_path: &OsStr) -> io::Result<()> {
        self.sftp_session
            .symlink(conv_path(source_path), conv_path(destination_path))
            .await
            .map_err(io::Error::other)
    }

    async fn create_hard_link(&self, source_path: &OsStr, destination_path: &OsStr) -> io::Result<()> {
        match self
            .sftp_session
            .hardlink(conv_path(source_path), conv_path(destination_path))
            .await
            .map_err(io::Error::other)?
        {
            true => Ok(()),
            false => Err(io::Error::other("SFTP server doesn't support the hardlink extension")),
        }
    }

    async fn read_link(&self, link_path: &OsStr) -> io::Result<OsString> {
        self.sftp_session
            .read_link(conv_path(link_path))
            .await
            .map(|path| path.into())
            .map_err(io::Error::other)
    }

    async fn set_permissions(&self, path: &OsStr, permissions: LinuxPermissions) -> io::Result<()> {
        self.sftp_session
            .set_metadata(
                conv_path(path),
                Metadata {
                    size: None,
                    uid: None,
                    user: None,
                    gid: None,
                    group: None,
                    permissions: Some(permissions.bits()),
                    atime: None,
                    mtime: None,
                },
            )
            .await
            .map_err(io::Error::other)
    }

    async fn remove_file(&self, path: &OsStr) -> io::Result<()> {
        self.sftp_session
            .remove_file(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn create_dir(&self, path: &OsStr) -> io::Result<()> {
        self.sftp_session
            .create_dir(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn create_dir_recursively(&self, path: &OsStr) -> io::Result<()> {
        let mut current_path = String::new();
        let mut previous_existed = false;

        let path_buf = PathBuf::from(path);
        for component in path_buf.components() {
            if let Component::Normal(os_component_value) = component {
                let component_value = os_component_value
                    .to_str()
                    .ok_or_else(|| io::Error::other("could not convert os_str to str"))?;

                current_path.push('/');
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
        }

        Ok(())
    }

    async fn list_dir(&self, path: &OsStr) -> io::Result<Vec<LinuxDirEntry>> {
        let read_dir = self
            .sftp_session
            .read_dir(conv_path(path))
            .await
            .map_err(io::Error::other)?;
        let entries = read_dir
            .map(|dir_entry| LinuxDirEntry {
                name: dir_entry.file_name().into(),
                file_type: dir_entry.file_type().into(),
                path: PathBuf::from(path).join(Path::new(&dir_entry.file_name())).into(),
            })
            .collect::<Vec<_>>();

        Ok(entries)
    }

    async fn remove_dir(&self, path: &OsStr) -> io::Result<()> {
        self.sftp_session
            .remove_dir(conv_path(path))
            .await
            .map_err(io::Error::other)
    }

    async fn remove_dir_recursively(&self, path: &OsStr) -> io::Result<()> {
        let str_path = path
            .to_str()
            .ok_or(io::Error::other("could not convert &OsStr to str"))?;

        run_fs_command(self, format!("rm -r {}", str_path)).await.map(|_| ())
    }

    async fn get_metadata(&self, path: &OsStr) -> io::Result<LinuxFileMetadata> {
        self.sftp_session
            .metadata(conv_path(path))
            .await
            .map(|attrs| attrs.into())
            .map_err(io::Error::other)
    }

    async fn get_symlink_metadata(&self, path: &OsStr) -> io::Result<LinuxFileMetadata> {
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

async fn run_fs_command<H>(instance: &RusshLinux<H>, command: String) -> io::Result<Option<u32>>
where
    H: client::Handler,
{
    let mut chan = instance.fs_channel_mutex.lock().await;
    let exec_result = chan.exec(true, command).await;
    if let Err(err) = exec_result {
        return Err(io::Error::other(err));
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

fn conv_path(path: &OsStr) -> String {
    String::from(path.to_str().unwrap())
}

impl Into<LinuxFileMetadata> for FileAttributes {
    fn into(self) -> LinuxFileMetadata {
        LinuxFileMetadata {
            file_type: Some(self.file_type().into()),
            size: self.size,
            permissions: match self.permissions {
                Some(bit) => Some(LinuxPermissions::from_bits_retain(bit)),
                None => None,
            },
            modified_time: self.modified().ok(),
            accessed_time: self.accessed().ok(),
            created_time: None,
            user_id: self.uid,
            user_name: self.user,
            group_id: self.gid,
            group_name: self.group,
        }
    }
}
