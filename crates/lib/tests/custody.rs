use runseal::custody::Private;
use std::fs::File;
use std::path::PathBuf;

struct Fixture {
    _root: tempfile::TempDir,
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        #[cfg(windows)]
        let root =
            tempfile::tempdir_in(std::env::var_os("ProgramData").expect("ProgramData")).unwrap();
        #[cfg(unix)]
        let root = tempfile::tempdir_in(std::fs::canonicalize("/tmp").unwrap()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let folder = root.path().join("private");
        Private::current().folder(&folder).unwrap();
        Self {
            path: folder.join("writer.lock"),
            _root: root,
        }
    }
}

#[test]
fn conflict() {
    let fixture = Fixture::new();
    let held = Private::current().hold(&fixture.path).unwrap();
    assert!(Private::current().hold(&fixture.path).is_err());
    let other = fixture.path.with_file_name("other.lock");
    let independent = Private::current().hold(&other).unwrap();
    drop(independent);
    drop(held);
}

#[test]
fn reentry() {
    let fixture = Fixture::new();
    let held = Private::current().hold(&fixture.path).unwrap();
    let copied = held.file().try_clone().unwrap();
    drop(held);
    let next = Private::current().hold(&fixture.path).unwrap();
    assert!(Private::current().hold(&fixture.path).is_err());
    assert_eq!(copied.metadata().unwrap().len(), 0);
    assert!(
        Private::current()
            .read(&fixture.path, 8)
            .unwrap()
            .is_empty()
    );
    drop(next);
    drop(copied);
}

#[test]
fn release() {
    let fixture = Fixture::new();
    let held = Private::current().hold(&fixture.path).unwrap();
    let copied = held.file().try_clone().unwrap();
    held.release().unwrap();
    let next = Private::current().hold(&fixture.path).unwrap();
    assert_eq!(copied.metadata().unwrap().len(), 0);
    drop(next);
    drop(copied);
}

#[test]
fn unwind() {
    let fixture = Fixture::new();
    let failure = std::panic::catch_unwind(|| {
        let held = Private::current().hold(&fixture.path).unwrap();
        let copied = held.file().try_clone().unwrap();
        std::panic::panic_any(copied);
    })
    .unwrap_err();
    assert!(failure.downcast_ref::<File>().is_some());
    let next = Private::current().hold(&fixture.path).unwrap();
    drop(next);
    drop(failure);
}
