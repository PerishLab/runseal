#![allow(unused_imports)]
use std::process::Command;

use tempfile::TempDir;

pub fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

pub struct Shell;

impl Shell {
    #[cfg(unix)]
    pub fn args(script: &str) -> Vec<String> {
        vec!["bash".into(), "--".into(), "-lc".into(), script.into()]
    }

    #[cfg(windows)]
    pub fn args(script: &str) -> Vec<String> {
        vec![
            "pwsh".into(),
            "--".into(),
            "-NoProfile".into(),
            "-Command".into(),
            script.into(),
        ]
    }
}

pub struct Script;

impl Script {
    #[cfg(unix)]
    pub fn env(key: &str) -> String {
        format!("printf '%s' \"${key}\"")
    }

    #[cfg(windows)]
    pub fn env(key: &str) -> String {
        format!("[Console]::Write($env:{key})")
    }

    #[cfg(unix)]
    pub fn profile() -> String {
        "printf '%s|%s' \"$RUNSEAL_TEST_VALUE\" \"$(basename \"$RUNSEAL_PROFILE_PATH\")\"".into()
    }

    #[cfg(windows)]
    pub fn profile() -> String {
        "[Console]::Write(\"$env:RUNSEAL_TEST_VALUE|$(Split-Path -Leaf $env:RUNSEAL_PROFILE_PATH)\")"
            .into()
    }

    #[cfg(unix)]
    pub fn symlink(path: &std::path::Path) -> String {
        format!("test -L {}", path.display())
    }
}

pub struct Probe;

impl Probe {
    #[cfg(unix)]
    pub fn write(path: &std::path::Path) {
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

pub struct Wrapper;

impl Wrapper {
    #[cfg(unix)]
    pub fn file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        dir.join(format!("{name}.sh"))
    }

    #[cfg(windows)]
    pub fn file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        dir.join(format!("{name}.cmd"))
    }

    #[cfg(unix)]
    pub fn name(name: &str) -> String {
        format!("{name}.sh")
    }

    #[cfg(windows)]
    pub fn name(name: &str) -> String {
        format!("{name}.cmd")
    }

    #[cfg(unix)]
    pub fn write(path: &std::path::Path, label: &str) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            format!(
                "#!/usr/bin/env sh\nprintf '{}|%s|%s|%s|' \"$1\" \"$RUNSEAL_WRAPPER_NAME\" \"$(basename \"$RUNSEAL_WRAPPER_FILE\")\"\n",
                label
            ),
        )
        .expect("wrapper should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("wrapper metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("wrapper should be executable");
    }

    #[cfg(windows)]
    pub fn write(path: &std::path::Path, label: &str) {
        std::fs::write(
            path,
            format!(
                "@echo off\r\n<nul set /p=\"{}|%1|%RUNSEAL_WRAPPER_NAME%|%~nx0|\"\r\nexit /b 0\r\n",
                label
            ),
        )
        .expect("wrapper should be written");
    }
}
