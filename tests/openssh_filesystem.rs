use std::path::Path;

use common::{gen_tmp_path, OpensshData};
use remoteify::filesystem::{LinuxFilesystem, LinuxOpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod common;

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    assert!(!test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_file() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create(&path).await.unwrap().close().await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_dir() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().create_dir(&path).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn t() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    let mut reader = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().read())
        .await
        .unwrap();
    let mut buf = String::new();
    reader.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}
