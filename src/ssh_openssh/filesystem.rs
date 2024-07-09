use std::{
    fs::Permissions,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use futures_util::StreamExt;
use openssh_sftp_client::{file::TokioCompatFile, metadata::FileType};
use unix_permissions_ext::UNIXPermissionsExt;

use crate::filesystem::{LinuxDirEntry, LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions};

use super::OpensshLinux;

#[async_trait]
impl LinuxFilesystem for OpensshLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        let sftp = self.sftp_mutex.lock().await;
        Ok(sftp.fs().metadata(path).await.is_ok())
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.create(path)
            .await
            .map_err(io::Error::other)?
            .close()
            .await
            .map_err(io::Error::other)?;
        Ok(())
    }

    async fn open_file(&self, path: &Path, open_options: &LinuxOpenOptions) -> io::Result<TokioCompatFile> {
        let sftp = self.sftp_mutex.lock().await;
        let mut actual_options = sftp.options();
        if open_options.is_read() {
            actual_options.read(true);
        }
        if open_options.is_write() {
            actual_options.write(true);
        }
        if open_options.is_append() {
            actual_options.append(true);
        }
        if open_options.is_create() {
            actual_options.create(true);
        }
        if open_options.is_truncate() {
            actual_options.truncate(true);
        }

        actual_options
            .open(path)
            .await
            .map_err(io::Error::other)
            .map(|file| file.into())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().rename(old_path, new_path).await.map_err(io::Error::other)
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        let sftp = self.sftp_mutex.lock().await;
        let mut old_file = sftp
            .options()
            .read(true)
            .open(old_path)
            .await
            .map_err(io::Error::other)?;
        let mut new_file = sftp
            .options()
            .write(true)
            .truncate(true)
            .open(new_path)
            .await
            .map_err(io::Error::other)?;
        old_file.copy_all_to(&mut new_file).await.map_err(io::Error::other)?;
        let offset = old_file.offset();
        old_file.close().await.map_err(io::Error::other)?;
        new_file.close().await.map_err(io::Error::other)?;

        Ok(Some(offset))
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().canonicalize(path).await.map_err(io::Error::other)
    }

    async fn create_symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs()
            .symlink(source_path, destination_path)
            .await
            .map_err(io::Error::other)
    }

    async fn create_hard_link(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs()
            .hard_link(source_path, destination_path)
            .await
            .map_err(io::Error::other)
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().read_link(link_path).await.map_err(io::Error::other)
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        let mut actual_permissions = openssh_sftp_client::metadata::Permissions::new();

        actual_permissions.set_execute_by_owner(permissions.executable_by_owner());
        actual_permissions.set_execute_by_group(permissions.executable_by_group());
        actual_permissions.set_execute_by_other(permissions.executable_by_other());

        actual_permissions.set_write_by_owner(permissions.writable_by_owner());
        actual_permissions.set_write_by_group(permissions.writable_by_group());
        actual_permissions.set_write_by_other(permissions.writable_by_other());

        actual_permissions.set_read_by_owner(permissions.readable_by_owner());
        actual_permissions.set_read_by_group(permissions.readable_by_group());
        actual_permissions.set_read_by_other(permissions.readable_by_other());

        sftp.fs()
            .set_permissions(path, actual_permissions)
            .await
            .map_err(io::Error::other)
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().remove_file(path).await.map_err(io::Error::other)
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().create_dir(path).await.map_err(io::Error::other)
    }

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()> {
        run_fs_command(
            &self,
            "mkdir",
            vec![
                "-p",
                path.to_str()
                    .ok_or(io::Error::other("path couldn't be converted to str"))?,
            ],
        )
        .await
    }

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>> {
        let sftp = self.sftp_mutex.lock().await;
        let read_dir = sftp.fs().open_dir(path).await.map_err(io::Error::other)?.read_dir();
        tokio::pin!(read_dir);
        let mut entries: Vec<LinuxDirEntry> = Vec::new();

        while let Some(dir_entry_result) = read_dir.next().await {
            let dir_entry = dir_entry_result.map_err(io::Error::other)?;
            let entry_path = path.join(dir_entry.filename());
            let entry_type: LinuxFileType = dir_entry
                .file_type()
                .ok_or(io::Error::other("file has no type"))?
                .into();
            let entry_name = dir_entry.filename().to_string_lossy().to_string();

            entries.push(LinuxDirEntry::new(entry_name, entry_type, entry_path));
        }

        Ok(entries)
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs().remove_dir(path).await.map_err(io::Error::other)
    }

    async fn remove_dir_recursively(&self, path: &Path) -> io::Result<()> {
        run_fs_command(
            &self,
            "rm",
            vec![
                "-r",
                path.to_str().ok_or(io::Error::other("could not convert path to str"))?,
            ],
        )
        .await
    }

    async fn get_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        let sftp = self.sftp_mutex.lock().await;
        let metadata = sftp.fs().metadata(path).await.map_err(io::Error::other)?;

        todo!()
        // Ok(LinuxFileMetadata::new(
        //     metadata.file_type().map(|file_type| file_type.into()),
        //     metadata.len(),
        //     metadata.permissions().map(|perms| perms.into()),
        //     modified_time,
        //     accessed_time,
        //     created_time,
        //     user_id,
        //     user_name,
        //     group_id,
        //     group_name,
        // ))
    }

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        todo!()
    }
}

async fn run_fs_command(instance: &OpensshLinux, program: &str, args: Vec<&str>) -> io::Result<()> {
    let mut child = instance.session.command(program);
    child.args(args);
    let status_code = child.status().await.map_err(io::Error::other)?;
    match status_code.success() {
        true => Ok(()),
        false => Err(io::Error::other(
            "underlying issued command exited with a non-zero status code",
        )),
    }
}

impl From<FileType> for LinuxFileType {
    fn from(value: FileType) -> Self {
        if value.is_file() {
            return LinuxFileType::File;
        }
        if value.is_dir() {
            return LinuxFileType::Dir;
        }
        if value.is_symlink() {
            return LinuxFileType::Symlink;
        }
        return LinuxFileType::Other;
    }
}
