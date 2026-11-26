use std::path::{Path, PathBuf};

use runseal::core::{config::Config, injections::Lifecycle, profile::Profile};
use tempfile::TempDir;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent should exist");
    }
    std::fs::write(path, body).expect("file should be written");
}

fn seat() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().expect("temp dir should exist");
    let root = temp.path().join("repo");
    let nested = root.join("sub/deep");
    std::fs::create_dir_all(&nested).expect("nested dir should exist");
    (temp, root, nested)
}

#[test]
fn section() {
    let (_temp, root, nested) = seat();
    write(
        &root.join("plumb.toml"),
        r#"
[identity]
ns = "demo"

[runseal.env.vars]
EXTRA = "local://extra"
RUNSEAL_REPO_TMP_DIR = "local://custom"

[runseal.argv]
demo = ["--cfg", "resource://demo.toml"]

[[runseal.symlink]]
source = "resource://src"
target = "local://dst"
"#,
    );
    let config = Config::build(None, Vec::new(), &nested).expect("config should load");
    assert!(config.manifest());
    assert_eq!(config.root, root);
    assert_eq!(
        config.path.as_deref(),
        Some(root.join("plumb.toml").as_path())
    );
    let profile = Profile::load(&config).expect("profile should load");
    assert_eq!(
        profile.env.vars.get("EXTRA").map(Path::new),
        Some(root.join(".local").join("extra").as_path()),
    );
    assert_eq!(
        profile
            .env
            .vars
            .get("RUNSEAL_REPO_LOCAL_DIR")
            .map(Path::new),
        Some(root.join(".local").as_path()),
    );
    assert_eq!(
        profile
            .env
            .vars
            .get("RUNSEAL_REPO_SECRETS_DIR")
            .map(Path::new),
        Some(root.join(".local").join("secrets").as_path()),
    );
    assert_eq!(
        profile.env.vars.get("RUNSEAL_REPO_TMP_DIR").map(Path::new),
        Some(root.join(".local").join("custom").as_path()),
    );
    let args = profile.argv.get("demo").expect("argv should exist");
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "--cfg");
    assert_eq!(
        Path::new(&args[1]),
        root.join(".runseal").join("resources").join("demo.toml")
    );
    assert_eq!(profile.symlink.len(), 1);
    assert_eq!(
        profile.symlink[0].source,
        root.join(".runseal").join("resources").join("src")
    );
    assert_eq!(profile.symlink[0].target, root.join(".local").join("dst"));
}

#[test]
fn defaults() {
    let (_temp, root, nested) = seat();
    write(&root.join("plumb.toml"), "[identity]\nns = \"demo\"\n");
    let config = Config::build(None, Vec::new(), &nested).expect("config should load");
    assert!(config.manifest());
    assert_eq!(config.root, root);
    assert_eq!(
        config.path.as_deref(),
        Some(root.join("plumb.toml").as_path())
    );
    let profile = Profile::load(&config).expect("profile should load");
    assert_eq!(profile.env.vars.len(), 3);
    assert_eq!(
        profile
            .env
            .vars
            .get("RUNSEAL_REPO_LOCAL_DIR")
            .map(Path::new),
        Some(root.join(".local").as_path()),
    );
    assert_eq!(
        profile
            .env
            .vars
            .get("RUNSEAL_REPO_SECRETS_DIR")
            .map(Path::new),
        Some(root.join(".local").join("secrets").as_path()),
    );
    assert_eq!(
        profile.env.vars.get("RUNSEAL_REPO_TMP_DIR").map(Path::new),
        Some(root.join(".local").join("tmp").as_path()),
    );
    Lifecycle::with(&profile, |patch| {
        assert_eq!(patch.set.len(), 3);
        assert!(patch.unset.is_empty());
        Ok(())
    })
    .expect("lifecycle should accept defaults");
}

#[test]
fn unset() {
    let (_temp, root, nested) = seat();
    write(
        &root.join("plumb.toml"),
        r#"
[runseal.env]
unset = ["RUNSEAL_REPO_LOCAL_DIR", "RUNSEAL_REPO_TMP_DIR"]

[runseal.env.vars]
CUSTOM = "local://custom"
"#,
    );
    let config = Config::build(None, Vec::new(), &nested).expect("config should load");
    let profile = Profile::load(&config).expect("profile should load");
    assert!(!profile.env.vars.contains_key("RUNSEAL_REPO_LOCAL_DIR"));
    assert!(!profile.env.vars.contains_key("RUNSEAL_REPO_TMP_DIR"));
    assert!(profile.env.vars.contains_key("RUNSEAL_REPO_SECRETS_DIR"));
    assert_eq!(
        profile.env.vars.get("CUSTOM").map(Path::new),
        Some(root.join(".local").join("custom").as_path()),
    );
    Lifecycle::with(&profile, |patch| {
        assert_eq!(patch.set.len(), 2);
        assert!(patch.unset.contains("RUNSEAL_REPO_LOCAL_DIR"));
        assert!(patch.unset.contains("RUNSEAL_REPO_TMP_DIR"));
        Ok(())
    })
    .expect("lifecycle should accept unset defaults");
}

#[test]
fn collision() {
    let (_temp, root, nested) = seat();
    write(
        &root.join("plumb.toml"),
        r#"
[runseal.env]
unset = ["RUNSEAL_REPO_LOCAL_DIR"]

[runseal.env.vars]
RUNSEAL_REPO_LOCAL_DIR = "local://"
"#,
    );
    let config = Config::build(None, Vec::new(), &nested).expect("config should load");
    let profile = Profile::load(&config).expect("profile should load");
    let error = Lifecycle::with(&profile, |_| Ok(())).expect_err("explicit set+unset must fail");
    assert!(error.to_string().contains("env validation failed"));
}

#[test]
fn fallback() {
    let (_temp, root, nested) = seat();
    write(&root.join("runseal.toml"), "[env.vars]\nONLY = \"yes\"\n");
    let config = Config::build(None, Vec::new(), &nested).expect("standalone should load");
    assert!(!config.manifest());
    assert_eq!(
        config.path.as_deref(),
        Some(root.join("runseal.toml").as_path())
    );
    let profile = Profile::load(&config).expect("profile should load");
    assert_eq!(profile.env.vars.len(), 1);
    assert_eq!(
        profile.env.vars.get("ONLY").map(String::as_str),
        Some("yes")
    );

    write(&root.join("plumb.toml"), "[identity]\nns = \"demo\"\n");
    let config = Config::build(None, Vec::new(), &nested).expect("fallback should load");
    assert!(!config.manifest());
    assert_eq!(
        config.path.as_deref(),
        Some(root.join("runseal.toml").as_path())
    );
    let profile = Profile::load(&config).expect("profile should load");
    assert_eq!(profile.env.vars.len(), 1);
    assert_eq!(
        profile.env.vars.get("ONLY").map(String::as_str),
        Some("yes")
    );
}

#[test]
fn conflict() {
    for body in ["[runseal]\n", "[runseal.env.vars]\nEXTRA = \"1\"\n"] {
        let (_temp, root, nested) = seat();
        write(&root.join("runseal.toml"), "[env.vars]\nONLY = \"yes\"\n");
        write(&root.join("plumb.toml"), body);
        let error = Config::build(None, Vec::new(), &nested)
            .expect_err("coexisting plumb.toml [runseal] and runseal.toml must fail");
        let message = error.to_string();
        assert!(message.contains("plumb.toml"), "{message}");
        assert!(message.contains("runseal.toml"), "{message}");
    }
}

#[test]
fn malformed() {
    for body in ["[identity", "runseal = \"invalid\"\n"] {
        let (_temp, root, nested) = seat();
        write(&root.join("plumb.toml"), body);
        assert!(Config::build(None, Vec::new(), &nested).is_err());

        write(&root.join("runseal.toml"), "");
        assert!(Config::build(None, Vec::new(), &nested).is_err());
    }

    let (_temp, root, nested) = seat();
    write(&root.join("plumb.toml"), "[runseal]\nunknown = 1\n");
    let config = Config::build(None, Vec::new(), &nested).expect("config should discover manifest");
    assert!(Profile::load(&config).is_err());
}

#[test]
fn named() {
    let (_temp, root, nested) = seat();
    write(&root.join("plumb.toml"), "[identity");
    write(
        &root.join("runseal.release.toml"),
        "[env.vars]\nMODE = \"release\"\n",
    );
    let config =
        Config::build(Some("release".to_string()), Vec::new(), &nested).expect("named should load");
    assert!(!config.manifest());
    assert_eq!(
        config.path.as_deref(),
        Some(root.join("runseal.release.toml").as_path()),
    );
    let profile = Profile::load(&config).expect("profile should load");
    assert_eq!(profile.env.vars.len(), 1);
    assert_eq!(
        profile.env.vars.get("MODE").map(String::as_str),
        Some("release"),
    );

    let missing = Config::build(Some("absent".to_string()), Vec::new(), &nested)
        .expect_err("missing named profile must fail");
    assert!(
        missing
            .to_string()
            .contains("named profile not found: :absent")
    );
}
