use tokio::fs::try_exists;

use super::NativeLinux;
use crate::filesystem::LinuxFilesystem;
use std::{io::{self}, path::Path};

impl LinuxFilesystem for NativeLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        try_exists(path).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::fs::{create_dir, remove_dir, remove_file, File};
    use uuid::Uuid;

    use crate::{filesystem::LinuxFilesystem, native::NativeLinux};

    static LHF: NativeLinux = NativeLinux {};

    #[tokio::test]
    async fn exists_is_true_for_existent_file() {
        let path = make_tmp_path();
        File::create_new(&path).await.expect("Failed to create file");
        assert!(LHF.exists(path.as_ref()).await.expect("Failed call"));
        remove_file(&path).await.expect("Failed to delete file");
    }

    #[tokio::test]
    async fn exists_is_true_for_existent_directory() {
        let path = make_tmp_path();
        create_dir(&path).await.expect("Failed to create directory");
        assert!(LHF.exists(path.as_ref()).await.expect("Failed call"));
        remove_dir(&path).await.expect("Failed to delete directory");
    }

    fn make_tmp_path() -> String {
        format!("/tmp/{}", Uuid::new_v4())
    }
}
