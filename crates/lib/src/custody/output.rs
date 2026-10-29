use super::Private;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(crate) struct Reserved {
    file: Option<File>,
    path: PathBuf,
}

impl Private {
    pub(crate) fn output(&self, path: &Path) -> io::Result<Reserved> {
        Ok(Reserved {
            file: Some(self.reserve(path)?),
            path: path.to_owned(),
        })
    }
}

impl Reserved {
    pub(crate) fn commit(mut self, bytes: &[u8]) -> io::Result<()> {
        let file = self.file.as_mut().expect("reserved output is held");
        file.write_all(bytes)?;
        file.sync_all()?;
        self.file.take();
        Ok(())
    }
}

impl Drop for Reserved {
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Private;
    use std::fs::File;

    fn root() -> tempfile::TempDir {
        #[cfg(unix)]
        let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
        #[cfg(windows)]
        let root = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        root
    }

    #[test]
    fn delivery() {
        let root = root();
        let path = root.path().join("output");
        let held = Private::current().output(&path).unwrap();
        assert!(Private::current().output(&path).is_err());
        held.commit(b"public-fixture").unwrap();
        assert_eq!(
            Private::current().read(&path, 128).unwrap(),
            b"public-fixture"
        );
    }

    #[test]
    fn cleanup() {
        let root = root();
        let path = root.path().join("output");
        drop(Private::current().output(&path).unwrap());
        assert!(!path.exists());
        let mut held = Private::current().output(&path).unwrap();
        held.file = Some(File::open(&path).unwrap());
        assert!(held.commit(b"public-fixture").is_err());
        assert!(!path.exists());
    }
}
