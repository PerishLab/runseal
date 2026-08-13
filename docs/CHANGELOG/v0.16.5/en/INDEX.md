# Runseal v0.16.5

`@forgejo` now exposes the complete bounded path from a workflow run to a
selected job log without falling back to a generic HTTP request.

## Reachable jobs

`job list RUN` reports every job index, name, and status for a run. Pass the
chosen index to `job log RUN JOB`, optionally with the job-scoped `--watch`.

`task list RUN_NUMBER` now includes the run number and status in text output,
and `run show ID` can resolve a run number through the task listing when the
direct Actions identifier does not exist.
