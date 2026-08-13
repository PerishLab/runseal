# Migrating to Runseal v0.16.6

Existing profiles remain compatible. Replace guessed indexes or generic HTTP
calls with:

```text
runseal :PROFILE @forgejo task list RUN_NUMBER
runseal :PROFILE @forgejo job list RUN
runseal :PROFILE @forgejo job log RUN JOB --watch
```

Listing does not watch; only the selected job log does.
