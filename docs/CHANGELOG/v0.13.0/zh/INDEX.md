# Runseal v0.13.0

Runseal 采用共享 binary 交付闭包。`plumb.toml` 是唯一的产品发布声明；
构建矩阵、归档、manager、密封记录、验证、激活、smoke 和 tag 全部由
Plumb 与 Actions 负责。

canonical stable 是默认安装锚点；精确的 non-stable 候选仍然只是隔离的
验证工具。
