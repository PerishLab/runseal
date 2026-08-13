# Runseal v0.16.4

`@forgejo` 现在可以从 Actions job 的当前日志一直跟随到终态结论。

## 面向 job 的 watch

`job log RUN JOB --watch` 会先输出当前日志，在 job 运行期间只追加新内容，并以
job 的结论决定退出状态。成功与 skipped job 返回零；失败、取消与终态 blocked
job 返回非零。

`--poll-ms` 与 `--timeout-ms` 显式控制轮询边界。既有的一次性 `job log` 和分页
`task list RUN_NUMBER` 保持不变：tasks 用于发现 jobs，`--watch` 属于选定的 job
日志。
