# Migrating to Runseal v0.16.4

Existing profiles and one-shot `@forgejo` calls require no changes.

Replace scripts that repeatedly download a job log with the native follower:

```text
runseal :PROFILE @forgejo job log RUN JOB --watch
```

Use `--poll-ms N` or `--timeout-ms N` when the default one-second poll and
thirty-minute timeout do not fit the operation. Watched logs are a streaming
plain-text command and therefore do not combine with `--json`; structured
in-process calls remain one-shot.
