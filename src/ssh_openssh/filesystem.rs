use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
};

use async_trait::async_trait;
use futures_util::StreamExt;
use openssh_sftp_client::{
    file::TokioCompatFile,
    metadata::{FileType, MetaData, Permissions},
};

use crate::filesystem::{
    LinuxDirEntry, LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions, LinuxPermissions,
};

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

    async fn open_file(&self, path: &Path, open_options: &LinuxOpenOptions) -> io::Result<Pin<Box<TokioCompatFile>>> {
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
            actual_options.create(true); // specificity of this impl
            actual_options.truncate(true);
        }

        let tokio_compat_file = TokioCompatFile::new(actual_options.open(path).await.map_err(io::Error::other)?);
        Ok(Box::pin(tokio_compat_file))
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

    async fn set_permissions(&self, path: &Path, permissions: LinuxPermissions) -> io::Result<()> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs()
            .set_permissions(path, permissions.into())
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
        sftp.fs()
            .metadata(path)
            .await
            .map_err(io::Error::other)
            .map(|metadata| metadata.into())
    }

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        let sftp = self.sftp_mutex.lock().await;
        sftp.fs()
            .symlink_metadata(path)
            .await
            .map_err(io::Error::other)
            .map(|metadata| metadata.into())
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

impl Into<LinuxFileMetadata> for MetaData {
    fn into(self) -> LinuxFileMetadata {
        LinuxFileMetadata::new(
            self.file_type().map(|file_type| file_type.into()),
            self.len(),
            self.permissions().map(|perms| perms.into()),
            self.modified().map(|timestamp| timestamp.as_system_time()),
            self.accessed().map(|timestamp| timestamp.as_system_time()),
            None,
            self.uid(),
            None,
            self.gid(),
            None,
        )
    }
}

impl Into<Permissions> for LinuxPermissions {
    fn into(self) -> Permissions {
        let mut result = Permissions::new();

        result.set_execute_by_owner(self.contains(LinuxPermissions::OWNER_EXECUTE));
        result.set_execute_by_group(self.contains(LinuxPermissions::GROUP_EXECUTE));
        result.set_execute_by_other(self.contains(LinuxPermissions::OTHER_EXECUTE));

        result.set_write_by_owner(self.contains(LinuxPermissions::OWNER_WRITE));
        result.set_write_by_group(self.contains(LinuxPermissions::GROUP_WRITE));
        result.set_write_by_other(self.contains(LinuxPermissions::OTHER_WRITE));

        result.set_read_by_owner(self.contains(LinuxPermissions::OWNER_READ));
        result.set_read_by_group(self.contains(LinuxPermissions::GROUP_READ));
        result.set_read_by_other(self.contains(LinuxPermissions::OWNER_READ));

        result.set_suid(self.contains(LinuxPermissions::SET_UID));
        result.set_sgid(self.contains(LinuxPermissions::SET_GID));
        result.set_vtx(self.contains(LinuxPermissions::STICKY_BIT));

        result
    }
}

impl From<Permissions> for LinuxPermissions {
    fn from(value: Permissions) -> Self {
        let mut result = LinuxPermissions::empty();

        result.set(LinuxPermissions::SET_UID, value.suid());
        result.set(LinuxPermissions::SET_GID, value.sgid());
        result.set(LinuxPermissions::STICKY_BIT, value.svtx());

        result.set(LinuxPermissions::OWNER_EXECUTE, value.execute_by_owner());
        result.set(LinuxPermissions::GROUP_EXECUTE, value.execute_by_group());
        result.set(LinuxPermissions::OTHER_EXECUTE, value.execute_by_other());

        result.set(LinuxPermissions::OWNER_WRITE, value.write_by_owner());
        result.set(LinuxPermissions::GROUP_WRITE, value.write_by_group());
        result.set(LinuxPermissions::OTHER_WRITE, value.write_by_other());

        result.set(LinuxPermissions::OWNER_READ, value.read_by_owner());
        result.set(LinuxPermissions::GROUP_READ, value.read_by_group());
        result.set(LinuxPermissions::OTHER_READ, value.read_by_other());

        result
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
