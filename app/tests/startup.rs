use std::{collections::BTreeMap, path::Path, process::Command};

use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_runseal"))
}

fn map(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

#[test]
fn help() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).expect("empty cwd should be created");

    for (args, expected) in [
        (vec!["@profile", "--help"], "Usage: runseal @profile"),
        (vec!["@resources", "--help"], "Usage: runseal @resources"),
        (vec!["@resolve", "--help"], "Usage: runseal @resolve"),
        (vec!["@wrappers", "--help"], "Usage: runseal @wrappers"),
        (vec!["@which", "--help"], "Usage: runseal @which :<wrapper>"),
    ] {
        let output = bin()
            .current_dir(&cwd)
            .env("RUNSEAL_HOME", temp.path().join("home"))
            .args(args.clone())
            .output()
            .expect("runseal should run");

        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
        assert!(
            stdout.contains(expected),
            "expected stdout for {args:?} to contain {expected:?}, got {stdout:?}"
        );
    }
}

#[test]
fn missing() {
    let temp = TempDir::new().expect("temp dir should be created");
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).expect("empty cwd should be created");

    let output = bin()
        .current_dir(&cwd)
        .env("RUNSEAL_HOME", temp.path().join("home"))
        .arg("@profile")
        .output()
        .expect("runseal should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("no runseal profile found from"));
    assert!(stderr.contains("Hint: create runseal.toml here"));
}

#[test]
fn absolute() {
    let temp = TempDir::new().expect("temp dir should be created");
    let project = temp.path().join("project");
    std::fs::create_dir_all(&project).expect("project should be created");
    std::fs::write(
        project.join("runseal.toml"),
        "injections = []\n[resources]\nroot = \".resource\"\n",
    )
    .expect("profile should be written");

    let output = bin()
        .current_dir(&project)
        .env("RUNSEAL_HOME", "../home")
        .arg("@profile")
        .output()
        .expect("runseal should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let values = map(&stdout);
    for key in [
        "RUNSEAL_HOME",
        "RUNSEAL_PROFILE_HOME",
        "RUNSEAL_PROFILE_PATH",
    ] {
        let value = values.get(key).expect("profile output should include key");
        assert!(Path::new(value).is_absolute(), "{key} should be absolute");
    }

    let wrapper = values
        .get("RUNSEAL_WRAPPER_PATH")
        .expect("profile output should include wrapper path");
    for entry in std::env::split_paths(wrapper) {
        assert!(
            entry.is_absolute(),
            "wrapper path entry should be absolute: {}",
            entry.display()
        );
    }
}
