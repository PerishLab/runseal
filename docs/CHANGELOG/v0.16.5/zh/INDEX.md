# Runseal v0.16.5

`@forgejo` 现在可以从 workflow run 原生抵达选定 job 的日志，不再需要退回通用
HTTP 请求。

## 可达的 jobs

`job list RUN` 会列出 run 中每个 job 的索引、名称与状态。选定索引后传给
`job log RUN JOB`，并可使用面向 job 的 `--watch`。

`task list RUN_NUMBER` 的文本输出现在包含 run number 与状态；当直接的 Actions
标识不存在时，`run show ID` 也能通过 task 列表解析 run number。
