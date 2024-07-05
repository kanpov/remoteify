use common::{conv_path, get_tmp_path, TestData};
use lhf::filesystem::{LinuxFilesystem, LinuxOpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod common;

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    assert!(!test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_file() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    test_data.sftp.create(conv_path(&path)).await.unwrap();
    assert!(test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_dir() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    test_data.sftp.create_dir(conv_path(&path)).await.unwrap();
    assert!(test_data
        .implementation
        .exists(&path)
        .await
        .expect("Call failed"));
}

#[tokio::test]
async fn open_file_with_read_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    handle.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    handle.write_all(b"CON").await.unwrap();
    test_data.assert_file(&path, "CONtent").await;
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("first").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().append())
        .await
        .expect("Call failed");
    handle.write_all(b"second").await.unwrap();
    test_data.assert_file(&path, "firstsecond").await;
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("current").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    handle.write_all(b"next").await.unwrap();
    test_data.assert_file(&path, "next").await;
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let test_data = TestData::setup().await;
    let path = get_tmp_path();
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().create())
        .await
        .expect("Call failed");
    handle.write_all(b"content").await.unwrap();
    test_data.assert_file(&path, "content").await;
}
