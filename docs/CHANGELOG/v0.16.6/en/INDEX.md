# Runseal v0.16.6

`@forgejo` now provides a complete native route from a workflow run to the
correct job log.

`job list RUN` prints each job index, status, and name. The selected index can
be passed to `job log RUN JOB --watch`; `--watch` remains job-scoped.

Task text output includes its run number and status, and `run show` can resolve
a run number through the task listing when a direct Actions id is unavailable.
