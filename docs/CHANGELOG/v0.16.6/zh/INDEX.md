# Runseal v0.16.6

`@forgejo` 现在提供从 workflow run 到正确 job 日志的完整原生路径。

`job list RUN` 会输出每个 job 的索引、状态与名称。选定索引后可传给
`job log RUN JOB --watch`；`--watch` 仍然只面向 job。

task 文本输出包含 run number 与状态；直接 Actions id 不可用时，`run show`
也可以通过 task 列表解析 run number。
