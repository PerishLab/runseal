# Forgejo tool examples

Authentication uses `FORGEJO_TOKEN` when present. Otherwise runseal selects the
named or sole login in `~/.tea/tea.yml`. CI supplies `FORGEJO_URL` or
`GITHUB_SERVER_URL`; local use takes the URL from Tea.

```bash
runseal @tool forgejo repo get --repo PerishFire/runseal

runseal @tool forgejo pr find \
  --repo PerishFire/runseal \
  --head topic \
  --base main

runseal @tool forgejo pr guard \
  --repo PerishFire/runseal \
  --number 7

runseal @tool forgejo secret upsert \
  --repo PerishFire/runseal \
  --name RELEASE_PUBLISH_S3_SECRET_KEY \
  --value-env RELEASE_PUBLISH_S3_SECRET_KEY

runseal @tool forgejo workflow dispatch \
  --repo PerishFire/runseal \
  --workflow release-exact.yml \
  --ref main \
  --input ref=main \
  --input channel=beta \
  --input version=vX.Y.Z-beta.N
```

`run cancel` fails explicitly on Forgejo v15 because that release exposes no
supported REST endpoint for cancellation.
