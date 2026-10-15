use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

pub const ENABLED: &str = "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n";

pub struct Seat(tempfile::TempDir);

impl Seat {
    pub fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn run(&self, args: &[String]) -> Output {
        let home = self.path();
        Command::new(env!("CARGO_BIN_EXE_runseal"))
            .current_dir(home)
            .env("RUNSEAL_HOME", home)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", home.join("bin").display()),
            )
            .env("GODEBUG", "inherited-private")
            .args(args)
            .output()
            .expect("command")
    }

    pub fn configure(&self, config: &str) {
        std::fs::write(self.path().join("config.toml"), config).unwrap();
    }

    pub fn records(&self) -> Vec<Value> {
        let mut atoms = Vec::new();
        for entry in std::fs::read_dir(self.path().join("state/locus")).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name == "active.jsonl" || name.starts_with("sealed-") {
                let text = std::fs::read_to_string(path).unwrap();
                atoms.extend(
                    text.lines()
                        .map(|line| serde_json::from_str::<Value>(line).unwrap()),
                );
            }
        }
        atoms.sort_by_key(|atom| atom["at"].as_u64());
        atoms
    }

    pub fn facts(&self) -> Vec<Value> {
        let atoms = self.records();
        for atom in &atoms {
            if atom["context"]["runseal.exec"].is_null() {
                assert!(atom["context"]["runseal.program"].is_null());
                assert!(atom["context"]["runseal.target"].is_null());
                assert!(atom["context"]["runseal.wrapper"].is_null());
            }
        }
        atoms
            .into_iter()
            .filter(|atom| atom["context"]["runseal.exec"] == "prepared.v1")
            .inspect(|atom| {
                assert!(atom.get("source").is_none());
                assert!(serde_json::to_vec(atom).unwrap().len() < 4096);
            })
            .collect()
    }

    #[cfg(unix)]
    pub fn programs() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = Self::new();
        std::fs::create_dir(root.path().join("bin")).unwrap();
        for name in ["gh", "concord", "plumb", "private"] {
            let path = root.path().join("bin").join(name);
            std::fs::write(&path, "#!/bin/sh\nprintf '%s\\n' \"$@\" \"${GODEBUG-unset}\" \"${PRIVATE_MARKER-unset}\"\nprintf 'child-error\\n' >&2\nexit 23\n").unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        root
    }

    #[cfg(unix)]
    pub fn compare(&self, args: &[String]) {
        self.configure("[locus]\nenabled = false\n");
        let baseline = self.run(args);
        self.configure(ENABLED);
        let observed = self.run(args);
        assert_eq!(baseline.status.code(), observed.status.code());
        assert_eq!(baseline.stdout, observed.stdout);
        assert!(observed.stderr.ends_with(&baseline.stderr));
    }
}

pub fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
