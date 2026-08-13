# 迁移到 Runseal v0.16.1

在目录形式的 home profile 中建立操作者 Forgejo 座位：

```toml
# $RUNSEAL_HOME/profiles/perish/runseal.toml
[env.vars]
FORGEJO_URL = "https://git.perish.top"
FORGEJO_TOKEN_FILE = "local://secrets/forgejo"
```

只把 token 值放入
`$RUNSEAL_HOME/profiles/perish/.local/secrets/forgejo`。Runseal 不导入或改写
`tea.yml`，也不创建第二个凭据仓。

查看可用原子操作，并用一次读取证明座位：

```bash
runseal :perish @forgejo --help
runseal :perish @forgejo user show
```

Rust 消费者使用已发布的 registry package 与结构化入口：

```toml
runseal = { version = "0.16.1", registry = "perish" }
```

调用 `runseal::tool::call("forgejo", argv, vars)` 并消费返回的 `Reply`。在 `vars`
中提供 `FORGEJO_URL`，再提供 `FORGEJO_TOKEN_FILE` 或 `FORGEJO_TOKEN`；不要捕获
CLI stdout，也不要在消费者中重建 Forgejo 路由。

已有外部命令 profile 与 profile grammar 无需迁移。
