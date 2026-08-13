# Runseal v0.17.0

Runseal 现在包含原子化的 `@cloudflare` HTTP 方言。能力覆盖 account-owned 与
user-owned API token 的完整生命周期和权限发现，也覆盖 Worker service/domain
感知，以及 R2 bucket/custom-domain 的感知与拆除。

token create 和 roll 会预占一个新的 mode-0600 文件，并将返回值写入其中。
普通输出只包含路径与 SHA-256 指纹；库接口则通过禁止明文调试、退出时清零的
secret 类型交付该值。

共享 HTTP 层现在通过类型化 fault 保留 status、headers 与非 JSON body。
Cloudflare 安全读操作会对 429 和服务端错误进行有界重试，写操作仍只尝试一次。

选定的 Cloudflare 契约以官方 OpenAPI 仓库的
`4e2f140437b8e356fb28631ece09c26efd7e781c` commit 为固定依据。
