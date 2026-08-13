# Migrating to Runseal v0.16.5

Existing profiles and commands remain compatible.

Replace guessed job indexes or generic `get URL` calls with the native path:

```text
runseal :PROFILE @forgejo task list RUN_NUMBER
runseal :PROFILE @forgejo job list RUN
runseal :PROFILE @forgejo job log RUN JOB --watch
```

`--watch` remains a `job log` option; neither task nor job listing watches.
