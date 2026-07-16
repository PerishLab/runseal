#!/usr/bin/env sh
set -eu

MODE=${1:-}
RELEASE_VERSION=${2:-}
ARTIFACT_DIR=${3:-}

[ -n "$MODE" ] || { printf '%s\n' 'missing mode' >&2; exit 1; }
[ -n "$RELEASE_VERSION" ] || { printf '%s\n' 'missing release version' >&2; exit 1; }
[ -n "$ARTIFACT_DIR" ] || { printf '%s\n' 'missing artifact dir' >&2; exit 1; }
[ -d "$ARTIFACT_DIR" ] || { printf 'artifact dir missing: %s\n' "$ARTIFACT_DIR" >&2; exit 1; }

require_file() {
  [ -f "$ARTIFACT_DIR/$1" ] || { printf 'missing artifact: %s\n' "$1" >&2; exit 1; }
}

require_checksum_entry() {
  awk -v name="$1" 'NR > 1 && $NF == name { found = 1 } END { exit(found ? 0 : 1) }' \
    "$ARTIFACT_DIR/checksums.txt"
}

ensure_tar_contains() {
  tar tzf "$ARTIFACT_DIR/$1" | grep -Fxq "$2" || {
    printf 'missing %s in %s\n' "$2" "$1" >&2
    exit 1
  }
}

check_archive_members() {
  for tarball in $TARBALLS; do
    ensure_tar_contains "$tarball" "runseal"
  done
}

TARBALLS="runseal-x86_64-unknown-linux-gnu.tar.gz runseal-aarch64-apple-darwin.tar.gz runseal-x86_64-apple-darwin.tar.gz"
ASSETS="$TARBALLS runseal-x86_64-pc-windows-msvc.zip"

case "$MODE" in
  accept)
    require_file checksums.txt
    for asset in $ASSETS; do
      require_file "$asset"
    done
    version_line=$(sed -n 's/^VERSION: *//p' "$ARTIFACT_DIR/checksums.txt" | head -n 1)
    [ "$version_line" = "$RELEASE_VERSION" ] || {
      printf 'version mismatch: expected %s got %s\n' "$RELEASE_VERSION" "$version_line" >&2
      exit 1
    }
    for asset in $ASSETS; do
      require_checksum_entry "$asset" || {
        printf 'missing checksum entry: %s\n' "$asset" >&2
        exit 1
      }
    done
    ;;
  verify)
    check_archive_members
    ;;
  *)
    printf 'unknown mode: %s\n' "$MODE" >&2
    exit 1
    ;;
esac
