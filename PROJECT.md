# 项目说明（Fork）

这是 [Modrinth/code](https://github.com/Modrinth/code) 的 fork，目标是开发一个**定制化的 Minecraft 启动器**（基于 Modrinth App / Theseus）。

## 基本信息

- 位置：`E:\Code\modrinth`（Linux 工作区：`/home/wangwenjie/Projects/Ferriso-Launcher`）
- 上游：`Modrinth/code`（remote `upstream` = `git@github.com:Modrinth/code.git`）
- Fork：`GBLMX/code`（remote `origin` = `git@github.com:GBLMX/code.git`）
- 基线：上游 `main` @ 2026-08-25（ZIP 快照导入，`eae097b`，**与上游无共享 git 历史**）
- 分支：`win-work-20260923` 承载本地快照与 Windows 侧改动，已推送到 `origin`；`origin/main` 保持上游同步，不被本地快照覆盖
- 许可证：各 package 单独许可，见 `COPYING.md`

## 同步上游

本仓库由 ZIP 快照初始化，与上游无共同提交历史，首次合并需 `--allow-unrelated-histories`：

```bash
git fetch upstream
git merge upstream/main --allow-unrelated-histories   # 仅首次需要
# 之后每次同步：
git merge upstream/main
```

> 冲突处理以 `apps/app*`（启动器）和 `packages/app-lib` 为重，web 前端/后端可先接受上游。

## 开发命令

```bash
pnpm app:dev       # 启动器（Tauri 桌面端）；先在 packages/app-lib 拷贝 .env 模板
pnpm web:dev       # 网站前端；先在 apps/frontend 拷贝 .env 模板
pnpm storybook     # 组件库 (packages/ui)
```

改动提交前按需运行 `pnpm prepr:frontend:app`（见根 AGENTS.md）。

## 工具链

| 工具 | 版本 | 状态 |
|---|---|---|
| node | v24.18.0 | ✅ |
| pnpm | 11.24.0 | ✅ |
| Rust | 1.95.0（`rust-toolchain.toml` 指定） | ❌ **未安装**（`cargo`/`rustc`/`rustup` 不在 PATH） |

> 开发 Tauri 启动器（`apps/app`）和 Labrinth 后端需要 Rust。装完后 `rustup toolchain install 1.95.0`。

## 约定（务必遵守，见根 `AGENTS.md`）

- TAB 缩进，不用空格
- 不用 `=== 标题 ===` 式注释；代码自文档化，避免无谓内联注释
- 前端 lint 只用 `prepr` 命令，不用 `tsc`/`typecheck`
- 进入子项目前先读其 `AGENTS.md`（`apps/labrinth`、`apps/frontend`）
- 不新建非源码文件（脚本/SQL）除非明确要求
