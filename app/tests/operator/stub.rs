use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("app dir should have repo parent")
        .to_path_buf()
}

pub struct Git;

impl Git {
    pub fn write(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            r#"#!/bin/sh
set -eu
case "${1:-}" in
  --version)
    ;;
  rev-parse)
    if [ "${2:-}" = "--show-toplevel" ]; then
      pwd
    else
      exit 9
    fi
    ;;
  config)
    if [ "${2:-}" = "core.hooksPath" ] && [ "${3:-}" = ".runseal/hooks" ]; then
      exit 0
    fi
    if [ "${2:-}" = "--get" ] && [ "${3:-}" = "core.hooksPath" ]; then
      printf '%s\n' ".runseal/hooks"
      exit 0
    fi
    exit 9
    ;;
  *)
    exit 9
    ;;
esac
"#,
        )
        .expect("git stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("git stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("git stub should be executable");
    }
}

pub struct Script;

impl Script {
    pub fn write(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        std::fs::write(
            path,
            format!(
                r#"#!/bin/sh
set -eu
if [ "${{1:-}}" = "--version" ] && [ "${{0##*/}}" = "negentropy" ]; then
  printf '%s\n' 'negentropy {}'
fi
if [ "${{1:-}}" = "config" ] && [ "${{2:-}}" = "--get" ]; then
  if [ "${{3:-}}" = "core.hooksPath" ]; then
    printf '%s\n' ".runseal/hooks"
  fi
fi
exit 0
"#,
                "v0.0.0"
            ),
        )
        .expect("stub should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("stub metadata should be readable")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("stub should be executable");
    }
}
