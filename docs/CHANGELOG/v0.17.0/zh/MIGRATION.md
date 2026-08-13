# 迁移到 Runseal v0.17.0

既有 profile 保持兼容。Cloudflare seat 需要提供 account，并提供 API token
文件或 API token：

```toml
[env.vars]
CLOUDFLARE_ACCOUNT_ID = "account-id"
CLOUDFLARE_API_TOKEN_FILE = "local://secrets/cloudflare"
```

用 `@cloudflare --help` 查看完整原子 verb map。token create 与 roll 必须传入
`--value-file`，且目标文件不得已经存在。

Rust 消费方应继续使用 `default-features = false`。既有
`runseal::tool::call` 调用保持有效；需要传入内存 policy body 或接收 token
secret 时，使用 `runseal::tool::cloudflare::invoke`。

若代码曾用 struct literal 构造 `tool::Reply`，需要改用公开 constructor 和
accessor；secret 字段现在有意保持私有。
