# 迁移到 Runseal v0.16.6

既有 profiles 保持兼容。请用以下原生路径替代猜测索引或通用 HTTP 调用：

```text
runseal :PROFILE @forgejo task list RUN_NUMBER
runseal :PROFILE @forgejo job list RUN
runseal :PROFILE @forgejo job log RUN JOB --watch
```

列表不负责 watch；只有选定的 job 日志负责。
