# Runseal v0.16.3

Runseal 的结构化原生工具 library 现在可以由 Plumb 嵌入，不再形成 package 依赖环。

## 无环的 library 座位

默认 Runseal binary 仍包含 managed skill 操作。依赖 Plumb 的这部分集成现在属于可选
`managed-skill` feature。Rust 消费者选择 `default-features = false` 后，会获得
profile core、HTTP transport、结构化 `Reply` 与 `runseal::tool::call`，但不携带
Plumb 依赖。

Runseal 现在直接持有核心 profile 解析、home 发现与构建版本探针。profile grammar、
home 路径、优先级、managed skill 行为以及完整 `@forgejo` 命令表均未改变。
