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
  --name RUNSEAL_RELEASES_S3_SK \
  --value-env RUNSEAL_RELEASES_S3_SK

runseal @tool forgejo workflow dispatch \
  --repo PerishFire/runseal \
  --workflow release-beta.yml \
  --ref main \
  --input version_override=
```

`run cancel` fails explicitly on Forgejo v15 because that release exposes no
supported REST endpoint for cancellation.
