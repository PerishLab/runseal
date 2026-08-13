# Runseal v0.15.0

这是 0.15 线的首个 stable 版本。它把 0.14 只写成产品法、还不能安装的 skill
座位变成可发布的 brief。

## 一等 skill brief

Runseal 现在在 `skills/runseal` 下交付封闭的三分 brief。frontmatter 只写何时
启用：查看或应用 profile、编辑 `runseal.toml`、或改 Runseal 仓库。

控制面新增 `runseal skill`，用于受管的 install、upgrade、status、stage、list
与 uninstall。受管座位只收 stable。exact 非 stable brief 使用 `skill stage`，
并要求新路径以 `runseal` 结尾。

仓库以 `[[document]]` strategy 声明 brief，并设置 `[release].skill = true`。
打包与落位由机制派生。

## 仓库本地法留在 AGENTS.md

`AGENTS.md` 保留仓库本地操作法。操作语法在 brief 里。两边都不复述对方的
表面。
