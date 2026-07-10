#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
VERSION=${1:-}
CHANNEL=${2:-stable}

[ -n "$VERSION" ] || { printf '%s\n' 'missing release version' >&2; exit 1; }

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM

export HOME="$tmpdir/home"
export RUNSEAL_INSTALL_ROOT="$tmpdir/install"
export RUNSEAL_LOCAL_BIN_DIR="$tmpdir/bin"
mkdir -p "$HOME" "$RUNSEAL_INSTALL_ROOT" "$RUNSEAL_LOCAL_BIN_DIR"

sh "$ROOT/manage.sh" install --channel "$CHANNEL" --version "$VERSION" --retain=false
"$RUNSEAL_LOCAL_BIN_DIR/runseal" --version
RUNSEAL_HOME="$tmpdir/runseal-home" "$RUNSEAL_LOCAL_BIN_DIR/runseal" --profile "$ROOT/app/examples/runseal.toml" sh -- -c true
sh "$ROOT/manage.sh" uninstall --version "$VERSION"
[ ! -e "$RUNSEAL_INSTALL_ROOT/$VERSION" ] || { printf '%s\n' "version uninstall left $RUNSEAL_INSTALL_ROOT/$VERSION" >&2; exit 1; }

if [ "${SMOKE_LATEST:-}" = "1" ]; then
  rm -f "$RUNSEAL_LOCAL_BIN_DIR/runseal"
  rm -rf "$RUNSEAL_INSTALL_ROOT/latest-smoke"
  sh "$ROOT/manage.sh" install --channel "$CHANNEL" --install-root "$RUNSEAL_INSTALL_ROOT/latest-smoke" --retain=false
  "$RUNSEAL_LOCAL_BIN_DIR/runseal" --version
  RUNSEAL_HOME="$tmpdir/runseal-home-latest" "$RUNSEAL_LOCAL_BIN_DIR/runseal" --profile "$ROOT/app/examples/runseal.toml" sh -- -c true
  sh "$ROOT/manage.sh" uninstall --install-root "$RUNSEAL_INSTALL_ROOT/latest-smoke"
  [ ! -e "$RUNSEAL_INSTALL_ROOT/latest-smoke" ] || { printf '%s\n' "full uninstall left $RUNSEAL_INSTALL_ROOT/latest-smoke" >&2; exit 1; }
fi
