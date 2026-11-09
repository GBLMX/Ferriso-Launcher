# 项目说明（Fork）

这是 [Modrinth/code](https://github.com/Modrinth/code) 的 fork，目标是开发一个**定制化的 Minecraft 启动器**：保留 Theseus/Modrinth 后端能力，使用 Ferriso 原生 iced 前端，不依赖 Tauri WebView。

## 基本信息

- 位置：`E:\Code\modrinth`（Linux 工作区：`/home/gblmx/src/github.com/GBLMX/Ferriso-Launcher`）
- 上游：`Modrinth/code`（remote `upstream` = `git@github.com:Modrinth/code.git`）
- Fork：`GBLMX/Ferriso-Launcher`（remote `origin` = `git@github.com:GBLMX/Ferriso-Launcher.git`）
- 基线：上游 `main` @ 2026-08-25（ZIP 快照导入，`eae097b`，**与上游无共享 git 历史**）
- 分支：`win-work-20260923` 承载本地快照与 Windows 侧改动，已推送到 `origin`；`origin/main` 是 fork 的基线/文档分支，不覆盖该工作分支
- 许可证：各 package 单独许可，见 `COPYING.md`

## 当前实现状态

- 原生启动器：`apps/app-ferriso`，入口 `ferriso-launcher`，使用 Ferriso `0.3.1`（git pin `802abee7f8efa6e9e3d2afbcd36ef22e4956e837`）。
- 领域状态、job/process/update/hosting 订阅和状态迁移由 `apps/app-ferriso/src/app.rs` 的 `FerrisoApp` 持有；`ferriso-core` 只提供 `App`/`Command`/`Subscription` 运行时契约。
- `apps/app-ferriso/src/backend.rs` 只负责启动前设置预读和 Theseus 事件流，不持有 UI 状态。
- Theseus 持久化设置是启动器设置的唯一来源；`ferriso-launcher.toml` / 环境变量只控制原生窗口和 Ferriso 自身选项。
- 键盘映射、系统主题探测和无障碍同步属于前端本地状态，不进入 actor。
- CurseForge：资源包和光影在官方 file 提供可下载 URL 时可直接安装；mods/modpacks 仍打开官方页面，依赖解析、手动下载识别和绕过限制尚未实现。
- 当前原生测试：`cargo +1.95.0 test -p ferriso-launcher --locked` 通过 45 个测试。

## 前后端边界

- 前端只发送语义消息并渲染不可变 `ViewModel`；不直接修改启动器领域状态。
- actor 负责定时器、后端事件流和异步命令；GUI 只拥有本地交互状态。
- Tauri 启动器与原生 Ferriso 启动器共用 `ModrinthApp` 数据目录，因此两者共享账户、实例和设置；原生端目前还没有 single-instance 锁，不能安全地与 Tauri 版同时运行。
- 未迁移的 Tauri-only 能力（深链、single-instance 转发、目录迁移取消、部分设置）需要在 native 侧补齐或明确禁用。

## 同步上游

本仓库由 ZIP 快照初始化，与上游无共同提交历史。`upstream/main` 已可拉取用于比较，但尚未做过实际合并；首次合并仍需 `--allow-unrelated-histories`：

```bash
git fetch upstream main
git merge upstream/main --allow-unrelated-histories   # 仅首次需要
# 之后每次同步：
git fetch upstream main
git merge upstream/main
```

当前快照与 `upstream/main` 的树差异很大（数千文件）；冲突处理以 `apps/app*`、`packages/app-lib` 和 CurseForge 相关迁移为重，web 前端/后端可先接受上游。

## 开发命令

```bash
# 原生 Ferriso 启动器
cargo +1.95.0 run -p ferriso-launcher

# Tauri 启动器（上游路径）
pnpm app:dev

# 网站前端
pnpm web:dev

# 组件库
pnpm storybook
```

原生启动器的配置位于 `$FERRISO_LAUNCHER_CONFIG` 或平台配置目录的 `config.toml`；环境变量使用 `FERRISO_LAUNCHER__*` 前缀。CurseForge 需要 `[curseforge] api_key` 或 `FERRISO_LAUNCHER__CURSEFORGE__API_KEY`。

改动提交前按需运行 `pnpm prepr:frontend:app`（见根 `AGENTS.md`）；原生 Rust 改动运行：

```bash
JAVA_HOME=/usr/lib/jvm/java-17-openjdk cargo +1.95.0 fmt --all -- --check
JAVA_HOME=/usr/lib/jvm/java-17-openjdk cargo +1.95.0 test -p ferriso-launcher --locked
JAVA_HOME=/usr/lib/jvm/java-17-openjdk cargo +1.95.0 clippy -p ferriso-launcher --all-targets --locked -- -D warnings
```

`packages/app-lib` 的 Gradle Java agent 期望 JDK 17；JDK 27 会触发 palantir-java-format 兼容错误。

## 工具链

| 工具 | 版本 | 状态 |
|---|---|---|
| node | 24.x（`.nvmrc`） | ✅ |
| pnpm | 10.33.2（`packageManager`） | ✅ |
| Rust | 1.95.0（`rust-toolchain.toml` 指定） | ✅ 已安装 |

## 约定（务必遵守，见根 `AGENTS.md`）

- TAB 缩进，不用空格
- 不用 `=== 标题 ===` 式注释；代码自文档化，避免无谓内联注释
- 前端 lint 只用 `prepr` 命令，不用 `tsc`/`typecheck`
- 进入子项目前先读其 `AGENTS.md`（`apps/labrinth`、`apps/frontend`）
- 不新建非源码文件（脚本/SQL）除非明确要求
