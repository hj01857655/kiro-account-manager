# Kiro 2API 本地化集成方案

## 1. 目标与概述
本方案旨在将 `kiro2api-deno` 项目作为一个功能模块完全整合进 `kiro-account-manager` 桌面应用中。
目标是让用户无需配置远程服务器，即可在本地一键启动 "OpenAI 转 Anthropic" 的 API 转换服务。

## 2. 架构设计：Sidecar (边车) 模式
我们将采用 Tauri 的 **Sidecar 模式**。
- **构建阶段**：使用 `deno compile` 将 TS 代码编译为对应平台的原生二进制可执行文件（如 `kiro-server-macos`）。
- **运行阶段**：Tauri 主进程 (Rust) 负责启动该二进制文件作为子进程。
- **通信**：
  - **控制流**：Rust -> 子进程 (启动/停止/环境变量注入)。
  - **数据流**：前端应用 -> HTTP 请求 (localhost) -> 子进程。
  - **日志流**：子进程 (Stdout/Stderr) -> Rust (事件监听) -> 前端 UI (日志展示)。

## 3. 目录结构重构
我们将 `kiro2api-deno` 移动至主项目下的 `server` 目录。

```text
kiro-account-manager/
├── src-tauri/
│   ├── src/
│   ├── bin/               <-- [新增] 存放编译后的 server 二进制文件
│   └── tauri.conf.json    <-- 配置 externalBin
├── server/                <-- [新增] 原 kiro2api-deno 的所有代码
│   ├── main.ts
│   ├── denos.json
│   └── ...
└── src/                   <-- 前端代码
```

## 4. 详细改造计划

### 4.1 Server 端改造 (Deno)
为了适应桌面环境运行，需对原 Deno 代码进行微调：
1.  **动态 KV 路径**：
    - 修改 `auth/kv_store.ts`。
    - 逻辑：优先读取环境变量 `KIRO_KV_PATH`。如果存在，则在该路径创建数据库文件；否则使用默认行为。
    - *目的：防止数据分散，将数据统一存储在操作系统的 AppData/Application Support 目录下。*
2.  **动态端口绑定**：
    - 确保 `main.ts` 能读取环境变量 `PORT`，避免端口冲突。
3.  **编译脚本**：
    - 在 `server/deno.json` 中添加 task，用于执行 `deno compile`。

### 4.2 宿主端改造 (Rust/Tauri)
1.  **Sidecar 配置**：
    - 在 `tauri.conf.json` 中注册 `externalBin`。
2.  **进程管理模块 (`server_process.rs`)**：
    - **启动逻辑**：
        - 获取用户应用数据目录 (App Data Dir)。
        - 组装 KV 数据库路径。
        - 使用 `Command::new_sidecar` 启动服务。
        - 注入环境变量：`KIRO_KV_PATH`, `PORT`, `KIRO_CLIENT_TOKEN` (如果需要)。
    - **生命周期管理**：
        - `start_server()`: 启动并保持句柄。
        - `stop_server()`: 优雅关闭子进程。
        - 监听子进程的 Stdout/Stderr，通过 `emit` 发送给前端。

### 4.3 前端改造 (React)
1.  **控制面板**：
    - 新增 "本地服务 (Local Server)" 页面。
    - 功能：启动/停止开关、端口配置输入框。
2.  **日志终端**：
    - 一个类似于 Terminal 的黑色窗口，实时显示从 Rust 传来的服务日志，方便排错。
3.  **状态同步**：
    - 顶栏或侧边栏显示一个小绿点/红点，指示本地服务是否运行中。

## 5. 实施步骤 (Checklist)

- [ ] **Step 1: 迁移**
    - 创建 `kiro-account-manager/server` 目录。
    - 将 `kiro2api-deno` 内容移动进去。
- [ ] **Step 2: Server 适配**
    - 修改 `kv_store.ts` 支持自定义路径。
    - 编写编译脚本，验证 `deno compile` 能够生成二进制文件。
- [ ] **Step 3: Tauri 配置**
    - 配置 `tauri.conf.json` 的 `bundle -> externalBin`。
    - 更新 `capabilities` 允许 shell execute (如果是 sidecar 模式)。
- [ ] **Step 4: Rust 逻辑实现**
    - 实现 `server_process.rs`。
    - 注册 Tauri Commands (`start_local_server`, `stop_local_server`).
- [ ] **Step 5: 前端实现**
    - 开发 UI 组件。
    - 联调测试。

## 6. 注意事项
- **二进制文件体积**：`deno compile` 会打包 runtime，生成的二进制文件约 60-80MB。这是可接受的代价，换取的是用户无需安装 Deno 环境。
- **跨平台构建**：如果需要发布 Windows 版本，需要在 Windows 环境下编译 server，或者配置 CI/CD 进行交叉编译。目前开发阶段优先保证 macOS 环境。
