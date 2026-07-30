# 迁移到 Runseal v0.13.0

runtime profile 与 wrapper 不需要迁移。

发布凭证现在统一使用仓库通用的 `RELEASE_PUBLISH_S3_*` 与
`RELEASE_ACTIVATE_S3_*` 名称。产品侧归档脚本和独立 release manifest
已经移除。
