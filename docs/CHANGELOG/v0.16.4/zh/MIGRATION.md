# 迁移到 Runseal v0.16.4

既有 profiles 与一次性 `@forgejo` 调用无需修改。

将反复下载 job 日志的脚本替换为原生 follower：

```text
runseal :PROFILE @forgejo job log RUN JOB --watch
```

默认一秒轮询与三十分钟超时不适合当前操作时，可传入 `--poll-ms N` 或
`--timeout-ms N`。watch 日志属于流式纯文本命令，因此不能与 `--json` 组合；
结构化进程内调用仍保持一次性语义。
