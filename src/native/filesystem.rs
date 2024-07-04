use async_trait::async_trait;
use tokio::fs::{try_exists, File};

use super::NativeLinux;
use crate::filesystem::LinuxFilesystem;
use std::{
    io::{self},
    path::Path,
};

#[async_trait]
impl LinuxFilesystem for NativeLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        try_exists(&path).await
    }

    async fn get_file_writer(&self, path: &Path) -> io::Result<File> {
        let file = match File::options().write(true).open(path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        Ok(file)
    }

    async fn get_file_writer_for_append(&self, path: &Path) -> io::Result<File> {
        let file = match File::options().append(true).open(path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        Ok(file)
    }

    async fn get_file_reader(&self, path: &Path) -> io::Result<File> {
        let file = match File::options().read(true).open(path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        Ok(file)
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match File::create_new(&path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        drop(file);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{filesystem::LinuxFilesystem, native::NativeLinux};
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
