# Repository Guidelines

## Project Structure & Module Organization
- 前端：`src/`（React 18 + Vite + Tailwind 4 + Mantine 7），页面与组件位于 `components/`（按 features/layout/modals/shared/ui 分层），状态与 hooks 在 `contexts/`、`hooks/`，通用工具在 `utils/`、`lib/`，API 封装放 `api/`。
- 国际化：`src/locales/` 与 `locales/` 为翻译资源；仅保留简体中文界面。
- 后端：`src-tauri/src/`（Rust 2021，Tauri 2.x）。`commands/` 定义 IPC 指令，`providers/` 管理登录方式，`state.rs` 维护全局状态；打包配置在 `tauri.conf.json`，依赖管理在 `Cargo.toml`。
- 资产与构建产物：`src/assets/` 静态资源；`dist/` 为前端构建输出；`screenshots/` 用于文档截图；`docs/` 存放使用/发布文档。

## Build, Test, and Development Commands
- 安装依赖：`npm install`（使用 package-lock，不要改用 pnpm/yarn）。推荐 Node 20。
- 全量开发：`npm run tauri dev`（同时启动 Vite 前端和 Rust 后端，端口 1420）。
- 前端单独：`npm run dev`、`npm run build`、`npm run preview`。
- 后端单独（进入 `src-tauri/`）：`cargo build`、`cargo test`、`cargo clippy`。
- 发行构建：`npm run tauri build`；版本号保持 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三处同步。
- 版本管理：`npm run release:(patch|minor|major)` 生成 changelog；`npm run publish` 调用 `scripts/release.ps1`。
- i18n：`npm run extract` / `npm run compile`（Lingui）。

## Coding Style & Naming Conventions
- JavaScript/JSX 使用 2 空格缩进、单引号、尽量无分号；函数式组件 + hooks（命名以 `use` 开头），组件文件 PascalCase，例如 `AuthCallback.jsx`。
- 样式优先集中在 `src/index.css` 与 `contexts/ThemeContext.jsx`，避免在组件内硬编码颜色；复用主题 token，暗色模式必须校验对比度。
- 路由配置在 `routes.jsx`，新增页面需同时注册侧边栏导航。
- Rust 遵循 `rustfmt` 默认风格；模块路径 snake_case。

## Testing Guidelines
- 当前无前端单元测试；最少执行 `npm run build` 捕获打包/语法错误，UI 改动需手动检查亮/暗主题、下拉 hover/selected 可见性、占位符可读性。
- 后端改动执行 `cargo test`；需要并发/网络相关改动时补充针对性测试用例。
- PR 前记录已执行的命令输出；长跑命令设 60s 超时以防卡死。

## Commit & Pull Request Guidelines
- Commit 使用 Conventional Commits（feat/fix/perf/refactor/docs/style/test/build/ci/chore），示例：`feat: add kiro cli import notice`。保持中文描述更贴合上下文。
- PR 需包含：变更摘要、关联 Issue/任务、已跑指令列表、UI 变更的前后截图（放 `screenshots/` 或附图链）、风险/回滚说明。
- 发布前确认版本号三处同步，release tag 由 CI 生成；避免提交打包产物和本地配置（如 `~/.kiro-account-manager` JSON）。

## Security & Configuration Tips
- 不要提交密钥/Token；示例占位使用 `[REDACTED]`。Tauri 签名私钥仅在 CI secrets。
- `.tauri-updater-key` / `.tauri-updater-password` 仅本地使用，严禁提交到仓库。
- Kiro 数据文件默认位于 `~/.kiro/`，应用数据在 `~/.kiro-account-manager/`；调试时勿覆盖用户真实配置，先备份。
- Kiro IDE 必须关闭后再触发切号/重置机器 ID；代理变更需重启 IDE 才生效。

## Public Repo Sync
- 开发以私有仓库 `dev` 为主；公开仓库默认分支为 `public`，旧版本分支为 `v1.5.1`。
- 同步到公开仓库时，移除 `docs/` 等内部内容，但保留 `screenshots/`。
- 公开仓库 Release 工作流直接从公开仓库构建，需在 GitHub Actions 配置 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

## Agent-Specific Notes
- 遵循 UI 安全规则：暗色模式避免低对比度，统一输入占位符色，Select/MultiSelect 需显式 hover/选中样式；优先在主题层修复再到页面局部。
- 修改同一路径的并行任务需提前拆分，避免写冲突；提交前用 `npm run build && cargo test` 作为最小回归。
