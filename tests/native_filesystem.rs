use std::{
    fs::{Metadata, Permissions},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};

use common::{entries_contain, gen_nested_tmp_path, gen_tmp_path};
use remoteify::{
    filesystem::{LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions},
    native::NativeLinux,
};
use tokio::{
    fs::{
        create_dir, create_dir_all, metadata, read_to_string, remove_dir, remove_dir_all, remove_file, symlink,
        symlink_metadata, try_exists, write, File,
    },
    io::{AsyncReadExt, AsyncWriteExt},
};
mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let path = gen_tmp_path();
    assert!(!IMPL.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_file() {
    let path = gen_tmp_path();
    File::create_new(&path).await.unwrap().flush().await.unwrap();
    assert!(IMPL.exists(&path).await.expect("Call failed"));
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn exists_is_true_for_existing_dir() {
    let path = gen_tmp_path();
    create_dir(&path).await.unwrap();
    assert!(IMPL.exists(&path).await.expect("Call failed"));
    remove_dir(&path).await.unwrap();
}

#[tokio::test]
async fn open_file_with_read_should_work() {
    let path = gen_tmp_path();
    write(&path, b"content").await.unwrap();
    let mut handle = IMPL
        .open_file(&path, &LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    handle.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let path = gen_tmp_path();
    write(&path, b"content").await.unwrap();
    let mut handle = IMPL
        .open_file(&path, &LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    handle.write_all(b"CON").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "CONtent");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let path = gen_tmp_path();
    write(&path, "first").await.unwrap();
    let mut handle = IMPL
        .open_file(&path, &LinuxOpenOptions::new().append())
        .await
        .expect("Call failed");
    handle.write_all(b"second").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "firstsecond");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let path = gen_tmp_path();
    write(&path, "first").await.unwrap();
    let mut handle = IMPL
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    handle.write_all(b"second").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "second");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let path = gen_tmp_path();
    let mut handle = IMPL
        .open_file(&path, &LinuxOpenOptions::new().create().write())
        .await
        .expect("Call failed");
    handle.write_all(b"content").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "content");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn create_file_should_persist() {
    let path = gen_tmp_path();
    IMPL.create_file(&path).await.expect("Call failed");
    assert!(try_exists(&path).await.unwrap());
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn rename_file_should_persist() {
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    File::create_new(&old_path).await.unwrap();
    IMPL.rename_file(&old_path, &new_path).await.expect("Call failed");
    assert!(!try_exists(&old_path).await.unwrap());
    assert!(try_exists(&new_path).await.unwrap());
    remove_file(&new_path).await.unwrap();
}

#[tokio::test]
async fn copy_file_should_persist() {
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    write(&old_path, "content").await.unwrap();
    IMPL.copy_file(&old_path, &new_path).await.expect("Call failed");
    assert_eq!(read_to_string(&new_path).await.unwrap(), "content");
    remove_file(&old_path).await.unwrap();
    remove_file(&new_path).await.unwrap();
}

#[tokio::test]
async fn canonicalize_should_perform_operation() {
    let canonicalized_path = IMPL
        .canonicalize(Path::new("/tmp/../tmp/../tmp"))
        .await
        .expect("Call failed");
    assert_eq!(canonicalized_path.to_str().unwrap(), "/tmp");
}

#[tokio::test]
async fn symlink_should_establish_link() {
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    write(&src_path, "").await.unwrap();
    IMPL.create_symlink(&src_path, &dst_path).await.expect("Call failed");
    assert!(try_exists(&dst_path).await.unwrap());
    assert!(symlink_metadata(&dst_path).await.is_ok());
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn hard_link_should_establish_link() {
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    write(&src_path, "content").await.unwrap();
    IMPL.create_hard_link(&src_path, &dst_path).await.expect("Call failed");
    assert!(try_exists(&dst_path).await.unwrap());
    assert_eq!(read_to_string(&dst_path).await.unwrap(), "content");
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn read_link_should_return_correct_location() {
    let src_path = gen_tmp_path();
    let dst_path = gen_tmp_path();
    write(&src_path, "").await.unwrap();
    symlink(&src_path, &dst_path).await.unwrap();
    assert_eq!(src_path, IMPL.read_link(&dst_path).await.unwrap());
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn set_permissions_should_perform_update() {
    let path = gen_tmp_path();
    write(&path, "content").await.unwrap();
    IMPL.set_permissions(&path, Permissions::from_mode(777)).await.unwrap();
    let meta = metadata(&path).await.unwrap();
    assert_eq!(meta.permissions().mode(), 33545);
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn remove_file_should_persist_changes() {
    let path = gen_tmp_path();
    write(&path, "content").await.unwrap();
    IMPL.remove_file(&path).await.expect("Call failed");
    assert!(!try_exists(&path).await.unwrap());
}

#[tokio::test]
async fn create_dir_should_persist() {
    let path = gen_tmp_path();
    IMPL.create_dir(&path).await.expect("Call failed");
    assert!(try_exists(&path).await.unwrap());
    assert!(metadata(&path).await.unwrap().is_dir());
    remove_dir(&path).await.unwrap();
}

#[tokio::test]
async fn create_dir_recursively_should_persist() {
    let path = gen_nested_tmp_path();
    IMPL.create_dir_recursively(&path).await.expect("Call failed");
    assert!(try_exists(&path).await.unwrap());
    remove_dir_all(&path.parent().unwrap()).await.unwrap();
}

#[tokio::test]
async fn list_dir_returns_correct_results() {
    let file_path = gen_tmp_path();
    write(&file_path, "content").await.unwrap();
    let dir_path = gen_tmp_path();
    create_dir(&dir_path).await.unwrap();
    let symlink_path = gen_tmp_path();
    symlink(&file_path, &symlink_path).await.unwrap();

    let entries = IMPL.list_dir(Path::new("/tmp")).await.expect("Call failed");

    entries_contain(&entries, LinuxFileType::File, &file_path);
    entries_contain(&entries, LinuxFileType::Dir, &dir_path);
    entries_contain(&entries, LinuxFileType::Symlink, &symlink_path);
}

#[tokio::test]
async fn remove_dir_should_persist() {
    let path = gen_tmp_path();
    create_dir(&path).await.unwrap();
    IMPL.remove_dir(&path).await.expect("Call failed");
    assert!(!try_exists(&path).await.unwrap());
}

#[tokio::test]
async fn remove_dir_recursively_should_persist() {
    let path = gen_nested_tmp_path();
    let parent_path = path.parent().unwrap();
    create_dir_all(&path).await.unwrap();
    IMPL.remove_dir_recursively(&parent_path).await.expect("Call failed");
    assert!(!try_exists(&parent_path).await.unwrap());
}

#[tokio::test]
async fn get_metadata_should_return_correct_result() {
    let path = gen_tmp_path();
    write(&path, b"content").await.unwrap();
    let expected_metadata = metadata(&path).await.unwrap();
    let actual_metadata = IMPL.get_metadata(&path).await.expect("Call failed");
    assert_metadata(expected_metadata, actual_metadata, LinuxFileType::File);
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn get_symlink_metadata_should_return_correct_result() {
    let src_path = gen_tmp_path();
    write(&src_path, "content").await.unwrap();
    let symlink_path = gen_tmp_path();
    symlink(&src_path, &symlink_path).await.unwrap();
    let expected_metadata = symlink_metadata(&symlink_path).await.unwrap();
    let actual_metadata = IMPL.get_symlink_metadata(&symlink_path).await.expect("Call failed");
    assert_metadata(expected_metadata, actual_metadata, LinuxFileType::Symlink);
    remove_file(&src_path).await.unwrap();
    remove_file(&symlink_path).await.unwrap();
}

fn assert_metadata(expected_metadata: Metadata, actual_metadata: LinuxFileMetadata, _file_type: LinuxFileType) {
    assert!(matches!(actual_metadata.file_type().unwrap(), _file_type));
    assert_eq!(actual_metadata.size().unwrap(), expected_metadata.size());
    assert_eq!(actual_metadata.permissions().unwrap(), expected_metadata.permissions());
    assert_eq!(
        actual_metadata.modified_time().unwrap(),
        expected_metadata.modified().unwrap()
    );
    assert_eq!(
        actual_metadata.accessed_time().unwrap(),
        expected_metadata.accessed().unwrap()
    );
    assert_eq!(
        actual_metadata.created_time().unwrap(),
        expected_metadata.created().unwrap()
    );
    assert_eq!(actual_metadata.user_id().unwrap(), expected_metadata.uid());
    assert_eq!(actual_metadata.user_name(), None);
    assert_eq!(actual_metadata.group_id().unwrap(), expected_metadata.gid());
    assert_eq!(actual_metadata.group_name(), None);
}
