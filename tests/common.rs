use std::path::PathBuf;

use uuid::Uuid;

pub fn get_tmp_path() -> PathBuf {
    PathBuf::from(format!("/tmp/{}", Uuid::new_v4().to_string()))
}