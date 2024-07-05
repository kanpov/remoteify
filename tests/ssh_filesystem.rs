use std::path::PathBuf;

use common::{get_tmp_path, TestData};
use lhf::filesystem::LinuxFilesystem;

mod common;

#[tokio::test]
async fn ssh_exists_false_for_missing_item() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    assert!(!test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

#[tokio::test]
async fn ssh_exists_true_for_existent_file() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    test_data.sftp.create(path_to_str(&path)).await.unwrap();
    assert!(test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

#[tokio::test]
async fn ssh_exists_true_for_existent_dir() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    test_data.sftp.create_dir(path_to_str(&path)).await.unwrap();
    assert!(test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

fn path_to_str(path: &PathBuf) -> String {
    path.to_str().unwrap().into()
}
