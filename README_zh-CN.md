# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/github/license/hj01857655/kiro-account-manager?color=orange" alt="License">
  <img src="https://img.shields.io/badge/QQ群-1020204332-12B7F5?logo=tencentqq" alt="QQ群">
</p>

<p align="center">
  <a href="README.md">English</a> | <a href="README_zh-CN.md">简体中文</a> | <a href="README_ru-RU.md">Русский</a>
</p>

<p align="center">
  <b>🚀 智能管理 Kiro IDE 账号，一键切换，配额监控</b>
</p>

---

## ✨ 功能特性

### 🔐 账号登录
- **Desktop OAuth** - 桌面端授权，支持 Google/GitHub/BuilderId
- **Web Portal OAuth** - 网页端授权，WebView 窗口内完成
- 两种方式互补，确保登录成功率

### 📊 账号展示
- 卡片网格布局，一目了然
- 配额进度条（主配额/试用/奖励）
- 订阅类型标识（Free/PRO/PRO+）
- Token 过期倒计时
- 状态高亮（正常/过期/封禁/当前使用）

### 🔄 一键切号
- 无感切换 Kiro IDE 账号
- 自动重置机器 ID
- 切换进度实时显示

### 📦 批量操作
- 批量刷新 / 批量删除
- JSON 导入导出
  - Social：refreshToken + provider
  - IdC：refreshToken + clientId + clientSecret
- SSO Token 批量导入
- 关键词搜索过滤

### 🔌 Kiro 配置
- **MCP 服务器** - 增删改查、启用/禁用
- **Powers** - 查看、安装、卸载
- **Steering 规则** - 查看、编辑

### ⚙️ 系统设置
- 四种主题（浅色/深色/紫色/绿色）
- AI 模型选择与锁定
- Token 自动刷新（可配置间隔）
- 切号自动重置机器 ID
- 机器 ID 绑定账号

### 🌐 浏览器与代理
- 自定义浏览器 / 自动检测
- 无痕模式启动
- HTTP 代理配置 / 自动检测

### 🔑 机器码管理
- 查看 / 备份 / 恢复 / 重置
- 支持 Windows / macOS

### 🖥️ IDE 集成
- 检测 Kiro IDE 运行状态
- 一键启动 / 关闭
- 自动同步代理和模型设置

## 📸 截图

| 首页 | 账号管理 |
|:---:|:---:|
| ![首页](screenshots/首页.png) | ![账号管理](screenshots/账号管理.png) |

| 登录 | 设置 |
|:---:|:---:|
| ![登录页](screenshots/登录页.png) | ![设置](screenshots/设置.png) |

## 📥 下载

[![Release](https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?style=flat-square)](https://github.com/hj01857655/kiro-account-manager/releases/latest)

👉 **[点击这里下载最新版本](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

| 平台 | 文件类型 | 说明 |
|------|----------|------|
| Windows | `.msi` | 推荐，双击安装 |
| Windows | `.exe` | NSIS 安装程序 |
| macOS | `.dmg` | 拖入 Applications |

## 💻 系统要求

- **Windows**: Windows 10/11 (64-bit)，需要 WebView2 (Win11 已内置)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon 通用)

## 🛠️ 技术栈

- **前端**: React 18 + Vite 5 + TailwindCSS 3 + Lingui (i18n)
- **后端**: Tauri 2.x + Rust + Tokio
- **图标**: Lucide React
- **存储**: JSON 文件本地存储

## 📁 数据存储

| 数据 | 路径 |
|------|------|
| 账号数据 | `%APPDATA%\.kiro-account-manager\accounts.json` |
| 应用设置 | `%APPDATA%\.kiro-account-manager\app-settings.json` |
| 机器码备份 | `%APPDATA%\.kiro-account-manager\machine-guid-backup.json` |
| MCP 配置 | `~/.kiro/settings/mcp.json` |
| Powers 注册表 | `~/.kiro/powers/registry.json` |
| Steering 规则 | `~/.kiro/steering/*.md` |

## 🔨 自行构建（Fork 用户）

如果你想自己构建应用：

1. **Fork** 本仓库到你的账号
2. 进入 **Actions** 标签页，启用工作流
3. 点击左侧 **"Build (Fork)"** 工作流
4. 点击 **"Run workflow"** 按钮开始构建
5. 等待约 15 分钟完成构建
6. 从完成的工作流运行中下载构建产物

> ⚠️ 自行构建的版本默认未签名，Windows 安装时会显示安全警告。
>
> 💡 如需启用代码签名，请在你的 fork 仓库添加 Secrets：`TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
>
> ⚠️ "Release" 工作流仅作者可用（需要私有仓库访问权限），Fork 用户请使用 "Build (Fork)" 工作流。

## ❓ 常见问题

**Q: 登录失败怎么办？**
A: 检查网络连接，尝试使用代理或切换登录方式。

**Q: Token 过期了怎么办？**
A: 点击刷新按钮，或开启自动刷新功能。

**Q: 如何备份账号？**
A: 使用导出功能，将账号数据保存为 JSON 文件。

**Q: 重置系统机器码失败怎么办？**
A: Windows 修改注册表需要管理员权限，请右键应用选择"以管理员身份运行"。

## 💬 交流反馈

- 💡 问题反馈、功能建议、使用交流
- 🐛 [提交 Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ 群：[Kiro Account Manager 交流群 (1020204332)](https://qm.qq.com/q/Vh7mUrNpa8)

<p align="center">
  <a href="https://qm.qq.com/q/Vh7mUrNpa8">
    <img src="https://img.shields.io/badge/QQ群-1020204332-12B7F5?style=for-the-badge&logo=tencentqq&logoColor=white" alt="QQ群">
  </a>
</p>

## ⚠️ 免责声明

本软件仅供学习交流使用，请勿用于商业用途。使用本软件所产生的任何后果由用户自行承担。

## 📄 开源协议

[GPL-3.0](LICENSE) - 修改后必须开源。

---

<p align="center">Made with ❤️ by hj01857655</p>
<p align="center">如果这个项目对你有帮助，请给个 ⭐！</p>
