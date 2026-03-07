use std::os::unix::fs::FileTypeExt;
use std::path::Path;

pub fn prepare_socket_path(socket_path: &Path) -> Result<(), String> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    if !socket_path.exists() {
        return Ok(());
    }

    let metadata = std::fs::symlink_metadata(socket_path).map_err(|err| err.to_string())?;
    if metadata.file_type().is_socket() {
        remove_socket_if_present(socket_path)?;
        return Ok(());
    }

    Err(format!(
        "refusing to remove non-socket path {}",
        socket_path.display()
    ))
}

pub fn remove_socket_if_present(socket_path: &Path) -> Result<(), String> {
    if !socket_path.exists() {
        return Ok(());
    }

    let metadata = std::fs::symlink_metadata(socket_path).map_err(|err| err.to_string())?;
    if metadata.file_type().is_socket() {
        std::fs::remove_file(socket_path).map_err(|err| err.to_string())?;
        return Ok(());
    }

    Err(format!(
        "refusing to remove non-socket path {}",
        socket_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use tempfile::tempdir;

    use super::{prepare_socket_path, remove_socket_if_present};

    #[test]
    fn removes_stale_socket_file() {
        let dir = tempdir().expect("tempdir");
        let socket_path = dir.path().join("rarag.sock");
        let _listener = UnixListener::bind(&socket_path).expect("bind socket");

        prepare_socket_path(&socket_path).expect("prepare socket path");

        assert!(!socket_path.exists());
    }

    #[test]
    fn rejects_non_socket_files() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("not-a-socket");
        std::fs::write(&file_path, "content").expect("write file");

        let err = prepare_socket_path(&file_path).expect_err("non-socket must be rejected");

        assert!(err.contains("refusing to remove non-socket path"));
        assert!(file_path.exists());
    }

    #[test]
    fn remove_socket_if_present_keeps_non_socket_files() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("owned-file");
        std::fs::write(&file_path, "content").expect("write file");

        let err = remove_socket_if_present(&file_path).expect_err("non-socket must be rejected");

        assert!(err.contains("refusing to remove non-socket path"));
        assert!(file_path.exists());
    }
}
