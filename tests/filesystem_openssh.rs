use std::ffi::OsString;

use common::{entries_contain, gen_nested_tmp_path, gen_tmp_path, OpensshData};
use openssh_sftp_client::metadata::MetaData;
use remoteify::filesystem::{LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions, LinuxPermissions};
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
async fn open_file_with_read_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    let mut reader = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    reader.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    writer.write_all(b"CON").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "CONtent").await;
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "first").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().append())
        .await
        .expect("Call failed");
    writer.write_all(b"second").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "firstsecond").await;
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "longfirstcontent").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    writer.write_all(b"second").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "second").await;
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().create())
        .await
        .expect("Call failed");
    writer.write_all(b"content").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "content").await;
}

#[tokio::test]
async fn create_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_file(&path).await.expect("Call failed");
    test_data.assert_file_exists(&path, true).await;
}

#[tokio::test]
async fn rename_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    test_data.sftp.fs().write(&old_path, "new content").await.unwrap();
    test_data
        .implementation
        .rename_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file_exists(&old_path, false).await;
    test_data.assert_file(&new_path, "new content").await;
}

#[tokio::test]
async fn copy_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    test_data.sftp.fs().write(&old_path, "content").await.unwrap();
    test_data
        .implementation
        .copy_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&old_path, "content").await;
    test_data.assert_file(&new_path, "content").await;
}

#[tokio::test]
async fn canonicalize_should_perform_operation() {
    let test_data = OpensshData::setup().await;
    assert_eq!(
        "/tmp",
        test_data
            .implementation
            .canonicalize(&OsString::from("/tmp/../tmp/../tmp"))
            .await
            .expect("Call failed")
            .to_str()
            .unwrap()
    );
}

#[tokio::test]
async fn symlink_should_establish_link() {
    let test_data = OpensshData::setup().await;
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    test_data.sftp.fs().write(&src_path, "content").await.unwrap();
    test_data
        .implementation
        .create_symlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    test_data.assert_file_exists(&dst_path, true).await;
    assert_eq!(test_data.sftp.fs().read_link(&dst_path).await.unwrap(), src_path);
}

#[tokio::test]
async fn hard_link_should_establish_link() {
    let test_data = OpensshData::setup().await;
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    test_data.sftp.fs().write(&src_path, "content").await.unwrap();
    test_data
        .implementation
        .create_hard_link(&src_path, &dst_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&dst_path, "content").await;
}

#[tokio::test]
async fn read_link_should_return_correct_location() {
    let test_data = OpensshData::setup().await;
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    test_data.sftp.fs().symlink(&src_path, &dst_path).await.unwrap();
    assert_eq!(
        src_path,
        test_data
            .implementation
            .read_link(&dst_path)
            .await
            .expect("Call failed")
    );
}

#[tokio::test]
async fn set_permissions_should_perform_update() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    test_data
        .implementation
        .set_permissions(
            &path,
            LinuxPermissions::empty()
                .union(LinuxPermissions::OWNER_READ)
                .union(LinuxPermissions::OTHER_EXECUTE),
        )
        .await
        .expect("Call failed");
    let permissions = test_data
        .sftp
        .fs()
        .metadata(&path)
        .await
        .unwrap()
        .permissions()
        .unwrap();
    assert!(permissions.read_by_owner());
    assert!(permissions.execute_by_other());
}

#[tokio::test]
async fn remove_file_should_persist_changes() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "sth").await.unwrap();
    test_data.implementation.remove_file(&path).await.expect("Call failed");
    test_data.assert_file_exists(&path, false).await;
}

#[tokio::test]
async fn create_dir_should_persist() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_dir(&path).await.expect("Call failed");
    test_data.assert_dir_exists(&path, true).await;
}

#[tokio::test]
async fn create_dir_recursively_should_persist() {
    let test_data = OpensshData::setup().await;
    let (parent_path, child_path) = gen_nested_tmp_path();
    test_data
        .implementation
        .create_dir_recursively(&child_path)
        .await
        .expect("Call failed");
    test_data.assert_dir_exists(&parent_path, true).await;
    test_data.assert_dir_exists(&child_path, true).await;
}

#[tokio::test]
async fn list_dir_returns_correct_results() {
    let test_data = OpensshData::setup().await;
    let file_path = gen_tmp_path();
    test_data.sftp.fs().write(&file_path, "content").await.unwrap();
    let dir_path = gen_tmp_path();
    test_data.sftp.fs().create_dir(&dir_path).await.unwrap();
    let symlink_path = gen_tmp_path();
    test_data.sftp.fs().symlink(&file_path, &symlink_path).await.unwrap();

    let entries = test_data
        .implementation
        .list_dir(&OsString::from("/tmp"))
        .await
        .expect("Call failed");
    entries_contain(&entries, LinuxFileType::File, &file_path);
    entries_contain(&entries, LinuxFileType::Dir, &dir_path);
    entries_contain(&entries, LinuxFileType::Symlink, &symlink_path);
}

#[tokio::test]
async fn remove_dir_should_persist() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().create_dir(&path).await.unwrap();
    test_data.implementation.remove_dir(&path).await.expect("Call failed");
    test_data.assert_dir_exists(&path, false).await;
}

#[tokio::test]
async fn remove_dir_recursively_should_persist() {
    let test_data = OpensshData::setup().await;
    let (parent_path, child_path) = gen_nested_tmp_path();
    test_data.sftp.fs().create_dir(&parent_path).await.unwrap();
    test_data.sftp.fs().create_dir(&child_path).await.unwrap();
    test_data
        .implementation
        .remove_dir_recursively(&parent_path)
        .await
        .expect("Call failed");
    test_data.assert_dir_exists(&parent_path, false).await;
}

#[tokio::test]
async fn get_metadata_should_return_correct_result() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    assert_metadata(
        test_data.sftp.fs().metadata(&path).await.unwrap(),
        test_data.implementation.get_metadata(&path).await.expect("Call failed"),
        LinuxFileType::File,
    );
}

#[tokio::test]
async fn get_symlink_metadata_should_return_correct_result() {
    let test_data = OpensshData::setup().await;
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    test_data.sftp.fs().symlink(&src_path, &dst_path).await.unwrap();
    assert_metadata(
        test_data.sftp.fs().symlink_metadata(&dst_path).await.unwrap(),
        test_data
            .implementation
            .get_symlink_metadata(&dst_path)
            .await
            .expect("Call failed"),
        LinuxFileType::Symlink,
    );
}

fn assert_metadata(expected_metadata: MetaData, actual_metadata: LinuxFileMetadata, _file_type: LinuxFileType) {
    assert!(matches!(actual_metadata.file_type().unwrap(), _file_type));
    assert_eq!(expected_metadata.len().unwrap(), actual_metadata.size().unwrap());
    assert_eq!(
        expected_metadata.modified().unwrap().as_system_time(),
        actual_metadata.modified_time().unwrap()
    );
    assert_eq!(
        expected_metadata.accessed().unwrap().as_system_time(),
        actual_metadata.accessed_time().unwrap()
    );
    assert_eq!(actual_metadata.created_time(), None);
    assert_eq!(expected_metadata.uid().unwrap(), actual_metadata.user_id().unwrap());
    assert_eq!(expected_metadata.gid().unwrap(), actual_metadata.group_id().unwrap());
    assert_eq!(actual_metadata.user_name(), None);
    assert_eq!(actual_metadata.group_name(), None);
}
