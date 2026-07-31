# Runseal v0.14.0

## 一个 profile，一个进程树

Runseal 现在只保留一个核心闭包：解析 profile、建立单次运行隔离，并执行一个命令
进程树。profile 只有三种能力：

- `env` 设置或移除子进程环境值；
- `argv` 在匹配的命令 token 后插入固定参数；
- `symlink` 在子进程生命周期内租用一个原本不存在的目标。

profile 不携带 command、默认动作、task graph、hook 行为、shell 表达式或编排。

## 冒号进入 profile 模式

没有冒号的调用始终属于 Runseal control plane，未知命令由 Clap 自然 quick-fail。
`:` 选择默认 profile，`:<name>` 选择具名 profile：

```bash
runseal profile
runseal : cargo test --locked
runseal :release cargo publish
```

profile 模式必须提供一个外部命令或原生 `@tool`。本版本不交付任何原生 tool：
只要 `env + argv + symlink` 能建立隔离，外部 CLI 就继续保持外部。

## 约定式 material

提交的惰性资源位于 `.runseal/resources`，忽略的 secret 与本地状态位于 `.local`。
profile 中的 `resource://` 与 `local://` 会以拒绝路径穿越的方式解析到这两个位置。

Runseal 不再承载 Deno、Sealkit、文件 wrapper、仓库 hook，也不引入替代脚本 runtime。
