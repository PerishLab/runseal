#!/usr/bin/env bash
set -euo pipefail

for name in RUNSEAL_RELEASES_PUBLIC_URL RELEASE_CHANNEL RELEASE_VERSION R2_METADATA_URL RUNNER_TEMP; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

run_id=${CI_RUN_ID:-local}
metadata="$RUNNER_TEMP/runseal-release-metadata.json"
curl -fsSL "$R2_METADATA_URL?run=$run_id" -o "$metadata"

public_url="${RUNSEAL_RELEASES_PUBLIC_URL%/}"
jq -e \
  --arg channel "$RELEASE_CHANNEL" \
  --arg version "$RELEASE_VERSION" \
  --arg unix "$public_url/manage.sh" \
  '
  (.channel == $channel)
  and (.releaseVersion == $version)
  and (.manage.unix == $unix)
  and (if .channel == "beta"
        then (.betaVersion == $version)
          and (.baseVersion | (type == "string") and (length > 0))
          and (.betaNumber | type == "number")
          and (("v" + .baseVersion + "-beta." + (.betaNumber | tostring)) == $version)
        else true end)
  and (.artifacts | to_entries | all(.value.url | (type == "string") and (length > 0)))
  ' "$metadata" >/dev/null || {
  echo "metadata validation failed" >&2
  exit 1
}

for url in $(jq -r '(.artifacts[].url), .manage.unix' "$metadata"); do
  curl -fsSI "$url" >/dev/null
done
