# 迁移到 Runseal v0.16.3

binary 用户与已有 profile 无需修改。managed skill 命令仍默认启用。

可能位于 Runseal managed-skill 依赖下层的进程内消费者使用纯 library 座位：

```toml
runseal = { version = "0", registry = "perish", default-features = false }
```

继续调用 `runseal::tool::call("forgejo", argv, vars)` 并消费结构化 `Reply`。只有当
消费方 binary 同时需要 Runseal managed skill 命令时，才启用默认 feature。
