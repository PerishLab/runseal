use std::{
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

pub struct Wrapper;

impl Wrapper {
    #[cfg(unix)]
    pub fn file(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.sh"))
    }

    #[cfg(windows)]
    pub fn file(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{name}.cmd"))
    }

    #[cfg(unix)]
    pub fn write(path: &Path, label: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, format!("#!/usr/bin/env sh\nprintf '{}'\n", label))
            .expect("wrapper should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("wrapper metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("wrapper should be executable");
    }

    #[cfg(windows)]
    pub fn write(path: &Path, label: &str) {
        Self::cmd(path, label);
    }

    #[cfg(windows)]
    pub fn cmd(path: &Path, label: &str) {
        std::fs::write(
            path,
            format!("@echo off\r\n<nul set /p=\"{}\"\r\nexit /b 0\r\n", label),
        )
        .expect("wrapper should be written");
    }
}

#[cfg(unix)]
pub struct Probe;

#[cfg(unix)]
impl Probe {
    pub fn write(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(path, "#!/usr/bin/env sh\nprintf '%s|' \"$@\"\n")
            .expect("probe should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("probe metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("probe should be executable");
    }
}

pub struct Fixture {
    pub _temp: TempDir,
    pub project: PathBuf,
    pub profile: PathBuf,
    pub home: PathBuf,
    pub wrappers: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should be created");
        let project = temp.path().join("project");
        let profile = project.join("runseal.toml");
        let home = temp.path().join("home");
        let wrappers = project.join(".runseal").join("wrappers");
        std::fs::create_dir_all(&wrappers).expect("project wrappers should be created");
        std::fs::create_dir_all(home.join("wrappers")).expect("home wrappers should be created");
        std::fs::write(
            &profile,
            "injections = []\n[resources]\nroot = \".resource\"\n",
        )
        .expect("profile should be written");
        Self {
            _temp: temp,
            project,
            profile,
            home,
            wrappers,
        }
    }

    pub fn run(&self, args: &[&str]) -> std::process::Output {
        bin()
            .current_dir(&self.project)
            .env("RUNSEAL_HOME", &self.home)
            .args(args)
            .output()
            .expect("runseal should run")
    }
}

pub struct Suffix;

impl Suffix {
    pub fn path(path: &Path, count: usize) -> PathBuf {
        path.components()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

pub struct Assert;

impl Assert {
    pub fn path(actual: &str, expected: &Path) {
        let expected = Suffix::path(expected, 4);
        assert!(
            Path::new(actual).ends_with(&expected),
            "expected {actual:?} to end with {}",
            expected.display()
        );
    }

    pub fn fails(fx: &Fixture, args: &[&str], expected: &str) {
        let output = fx.run(args);
        assert!(!output.status.success(), "{args:?} should fail");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
        assert!(
            stderr.contains(expected),
            "expected stderr for {args:?} to contain {expected:?}, got {stderr:?}"
        );
    }
}

#[cfg(windows)]
#[test]
pub fn cmd() {
    let fx = Fixture::new();
    let cmd = fx.wrappers.join("tool.cmd");
    let bat = fx.wrappers.join("tool.bat");
    Wrapper::cmd(&cmd, "cmd");
    Wrapper::cmd(&bat, "bat");

    let which = fx.run(&["@which", ":tool"]);
    assert!(which.status.success());
    let stdout = String::from_utf8(which.stdout).expect("stdout should be UTF-8");
    Assert::path(stdout.trim(), &cmd);

    let output = fx.run(&[":tool"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, "cmd");
}
