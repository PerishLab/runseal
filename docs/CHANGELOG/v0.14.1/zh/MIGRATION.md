# 迁移到 Runseal v0.14.1

本版本以 profile 闭包替换 wrapper host。

删除旧 wrapper 前，逐项完成归类：

- 通过 `runseal :[profile] <command>` 调用可被 profile 隔离的外部 CLI；
- 将通用 guard、init、land 与 release 行为交给底层 substrate 或规范 workflow；
- 将产品流程归还给拥有它的产品 CLI；
- 仅当一个第三方原子操作无法由 profile 三元组隔离时才引入 `@tool`。

不要把 wrapper 文件翻译成另一种脚本语言，也不要把它们改造成 Runseal command
list。

将 `runseal.toml` 重写为 profile 数据。旧 `resources`、`deno` 与 `injections`
section 不再被接受：

```toml
[env]
unset = ["AMBIENT_KEY"]

[env.vars]
TOKEN_FILE = "local://secrets/token"

[argv]
cargo = ["--locked"]

[[symlink]]
source = "resource://tool.conf"
target = "local://run/tool.conf"
```

将提交的惰性文件移动到 `.runseal/resources`，secret 保留在 `.local`；当行为已有
明确所有者后，删除 `.runseal/deno.json`、`.runseal/deno.lock`、通用 wrapper
与仓库自有 Git hook。

默认 profile 使用 `runseal : <command>`，具名 profile 使用
`runseal :<name> <command>`。不需要 `--` 分隔符；没有标记的 token 永远是 Runseal
内部命令。
