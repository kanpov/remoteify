use async_trait::async_trait;
use tokio::fs::{canonicalize, copy, rename, try_exists, File, OpenOptions};

use super::NativeLinux;
use crate::filesystem::LinuxFilesystem;
use std::{
    io,
    path::{Path, PathBuf},
};

#[async_trait]
impl LinuxFilesystem for NativeLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        try_exists(&path).await
    }

    async fn file_open_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().write(true), truncate).await
    }

    async fn file_open_append(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().append(true), truncate).await
    }

    async fn file_open_read(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().read(true), truncate).await
    }

    async fn file_open_read_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().read(true).write(true), truncate).await
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match File::create_new(path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        rename(old_path, new_path).await
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<u32> {
        copy(old_path, new_path).await.map(|x| u32::try_from(x).unwrap())
    }
    
    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        canonicalize(path).await
    }
}

async fn internal_open_file(
    path: &Path,
    open_options: &mut OpenOptions,
    truncate: bool,
) -> io::Result<File> {
    if truncate {
        open_options.truncate(true).open(path).await
    } else {
        open_options.open(path).await
    }
}

#[cfg(test)]
mod tests {
    use crate::filesystem::LinuxFilesystem;
    use crate::native::NativeLinux;
    use tokio::fs::{create_dir, remove_dir, remove_file, try_exists, File};
    use uuid::Uuid;

    static IMPL: NativeLinux = NativeLinux {};

    #[tokio::test]
    async fn exists_is_true_for_existent_file() {
        let path = make_tmp_path();
        drop(File::create_new(&path).await.unwrap());
        assert!(IMPL.exists(path.as_ref()).await.expect("Failed call"));
        remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn exists_is_true_for_existent_directory() {
        let path = make_tmp_path();
        create_dir(&path).await.unwrap();
        assert!(IMPL.exists(path.as_ref()).await.expect("Failed call"));
        remove_dir(&path).await.unwrap();
    }

    #[tokio::test]
    async fn exists_is_false_for_missing_file_or_directory() {
        let path = make_tmp_path();
        assert!(!IMPL.exists(path.as_ref()).await.expect("Failed call"));
    }

    #[tokio::test]
    async fn create_file_persists() {
        let path = make_tmp_path();
        IMPL.create_file(path.as_ref()).await.expect("Failed call");
        assert!(try_exists(&path).await.unwrap());
        remove_file(&path).await.unwrap();
    }

    fn make_tmp_path() -> String {
        format!("/tmp/{}", Uuid::new_v4())
    }
}
