# 迁移到 Runseal v0.16.5

既有 profiles 与命令保持兼容。

请用原生路径替代猜测 job 索引或通用 `get URL` 调用：

```text
runseal :PROFILE @forgejo task list RUN_NUMBER
runseal :PROFILE @forgejo job list RUN
runseal :PROFILE @forgejo job log RUN JOB --watch
```

`--watch` 仍然只属于 `job log`；task 与 job 列表都不负责 watch。
