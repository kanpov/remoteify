use async_trait::async_trait;
use tokio::fs::{try_exists, write, File};

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

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match File::create_new(&path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        drop(file);
        Ok(())
    }

    async fn write_text_to_file(&self, path: &Path, text: &String) -> io::Result<()> {
        write(path, text.as_bytes()).await?;
        Ok(())
    }

    async fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        write(path, bytes).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::fs::{create_dir, read_to_string, remove_dir, remove_file, try_exists, File};
    use uuid::Uuid;

    use crate::{filesystem::LinuxFilesystem, native::NativeLinux};

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

    #[tokio::test]
    async fn write_text_to_file_persists() {
        let path = make_tmp_path();
        drop(File::create_new(&path).await.unwrap());
        IMPL.write_text_to_file(path.as_ref(), &"content".to_string())
            .await
            .expect("Failed call");
        let actual_content = read_to_string(&path).await.expect("Failed to read");
        assert_eq!(actual_content, "content");
        remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn write_bytes_to_file_persists() {
        let content = "content".as_bytes();
        let path = make_tmp_path();
        drop(File::create_new(&path).await.unwrap());
        IMPL.write_bytes_to_file(path.as_ref(), &content)
            .await
            .expect("Failed call");
        let actual_content = read_to_string(&path).await.expect("Failed to read");
        assert_eq!(actual_content.as_bytes(), content);
        remove_file(&path).await.unwrap();
    }

    fn make_tmp_path() -> String {
        format!("/tmp/{}", Uuid::new_v4())
    }
}
