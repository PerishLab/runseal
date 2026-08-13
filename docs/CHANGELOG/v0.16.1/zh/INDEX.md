# Runseal v0.16.1

这是 0.16 线的首个 stable 版本。它交付 Runseal 的第一个原生原子工具，并把同一套
Forgejo 操作表面提供给进程内 Rust 消费者。

## Forgejo HTTP 成为原生工具

`runseal :perish @forgejo` 每次调用只执行一个具名 Forgejo HTTP 资源操作。
表面覆盖 issue、pull、review、提交状态、分支、保护规则、仓库退役、Actions
secret、workflow run、task 感知、job 日志、label 与未鉴权公共读取。列表走完
所有分页，并可用 `--limit` 截止。

具名 profile 是凭据座位。`@forgejo` 从已解析 profile 读取 `FORGEJO_URL` 与
`FORGEJO_TOKEN_FILE`，不创建 login 数据库。`@forgejo --help` 给出完整命令表。

## 一个方言，两个入口

`runseal` crate 现在与 binary、skill 一起发布到 `perish` Cargo registry。
`runseal::tool::call` 返回结构化 `Reply`，CLI 单独渲染结果。library 消费者因此
可以使用同一组 verb 与 Forgejo 方言，而不必解析 stdout 或重建路由。

workflow task 感知补齐了调用方区分 waiting、success、failure 与 blocked 所需的
HTTP 证据。
