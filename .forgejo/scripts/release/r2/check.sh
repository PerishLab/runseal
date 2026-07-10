#!/usr/bin/env bash
set -euo pipefail

for name in RUNSEAL_RELEASES_S3_AK RUNSEAL_RELEASES_S3_SK RUNSEAL_RELEASES_S3_BUCKET RUNSEAL_RELEASES_S3_URL RUNSEAL_RELEASES_PUBLIC_URL RELEASE_CHANNEL R2_ACCESS_PROBE_NAME; do
  if [ -z "${!name:-}" ]; then
    echo "$name is required" >&2
    exit 1
  fi
done

RUNNER_TEMP=${RUNNER_TEMP:-.local/tmp}
run_id=${CI_RUN_ID:-local}
sha=${CI_COMMIT:-unknown}
mkdir -p "$RUNNER_TEMP"
probe_file="$RUNNER_TEMP/runseal-r2-access.txt"
probe_key="$RELEASE_CHANNEL/.ci-access-check/$R2_ACCESS_PROBE_NAME.txt"
printf 'run=%s\nsha=%s\nchannel=%s\n' "$run_id" "$sha" "$RELEASE_CHANNEL" > "$probe_file"

AWS_ACCESS_KEY_ID="$RUNSEAL_RELEASES_S3_AK" \
AWS_SECRET_ACCESS_KEY="$RUNSEAL_RELEASES_S3_SK" \
AWS_DEFAULT_REGION=auto \
AWS_EC2_METADATA_DISABLED=true \
aws --endpoint-url "${RUNSEAL_RELEASES_S3_URL%/}" s3api put-object \
  --bucket "$RUNSEAL_RELEASES_S3_BUCKET" \
  --key "$probe_key" \
  --body "$probe_file" \
  --content-type "text/plain; charset=utf-8" \
  --cache-control "no-store" \
  --no-cli-pager >/dev/null
