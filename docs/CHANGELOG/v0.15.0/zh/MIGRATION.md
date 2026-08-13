# 迁移到 Runseal v0.15.0

本版本增加受管 skill 座位。profile 语法不变。

stable 激活后，用 stable 二进制安装或升级受管 skill：

```bash
runseal skill install
runseal skill upgrade
```

若默认座位上已有目录且未进入受管 ledger，先删除该目录再安装。`--force`
不能认领它。

若要在提升前评估候选，把 exact 非 stable skill stage 到以 `runseal` 结尾的
隔离路径。不要用 beta 替换受管 stable 座位。

profile、`runseal.toml` 与已存数据都不需要迁移。
