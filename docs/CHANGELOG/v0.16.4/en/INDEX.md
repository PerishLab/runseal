# Runseal v0.16.4

`@forgejo` can now follow one Actions job from its current log through its
terminal result.

## Job-scoped watch

`job log RUN JOB --watch` prints the current log, appends only newly observed
content while the job runs, and exits with the job's conclusion. Successful
and skipped jobs exit zero; failed, cancelled, and terminally blocked jobs exit
non-zero.

`--poll-ms` and `--timeout-ms` make the polling boundary explicit. The existing
one-shot `job log` and paged `task list RUN_NUMBER` operations are unchanged:
tasks discover jobs, while `--watch` belongs to a selected job log.
