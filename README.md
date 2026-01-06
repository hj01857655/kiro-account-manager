# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/github/license/hj01857655/kiro-account-manager?color=orange&t=20241224" alt="License">
  <img src="https://img.shields.io/badge/QQ群-1020204332-12B7F5?logo=tencentqq" alt="QQ群">
  <img src="https://img.shields.io/badge/语言-简体中文-red" alt="语言">
</p>

<p align="center">
  <b>🚀 智能管理 Kiro IDE 账号，一键切换，配额监控</b>
</p>

> **📢 语言支持**：从当前版本开始，本项目**仅支持简体中文界面**，已移除英文和俄语翻译。这样可以简化维护，专注于功能开发。

---

## ✨ 功能特性

### 🔐 账号登录
- **Desktop OAuth** - 桌面端授权，支持 Google/GitHub/BuilderId

### 📊 账号展示
- 🆕 卡片/列表双视图切换
- 配额进度条（主配额/试用/奖励）
- 订阅类型标识（Free/PRO/PRO+）
- Token 过期倒计时
- 状态高亮（正常/过期/封禁/当前使用）

### 🔄 一键切号
- 无感切换 Kiro IDE 账号
- 自动重置机器 ID
- 切换进度实时显示

### 📦 批量操作
- 🆕 批量刷新 / 批量删除 / 批量打标签
- JSON 导入导出（Social / IdC 格式）
- SSO Token 批量导入
- 🆕 卡密兑换（支持链动小铺等平台）
- 关键词搜索过滤

### 🏷️ 标签管理 🆕
- 自定义标签（名称/颜色）
- 批量设置标签
- 按标签筛选账号

### 🔍 高级筛选与排序 🆕
- 按订阅类型筛选（Free/PRO/PRO+）
- 按状态筛选（正常/封禁）
- 按使用率/添加时间/试用到期排序
- 三态排序（降序→升序→取消）

### 🔌 Kiro 配置
- **MCP 服务器** - 增删改查、启用/禁用
- **Steering 规则** - 查看、编辑

### ⚡ KiroGate
- 内置 OpenAI 兼容 API 代理服务
- 独立 Token 管理
- 支持多种 AI 客户端接入

### ⚙️ 系统设置
- 四种主题（浅色/深色/紫色/绿色）
- AI 模型选择与锁定
- 🆕 代码库索引开关
- 🆕 信任所有命令开关
- Token 自动刷新（可配置间隔）
- 切号自动重置机器 ID
- 🆕 隐私模式（隐藏敏感信息）

### 🌐 浏览器与代理
- 自定义浏览器 / 自动检测
- 无痕模式启动
- HTTP 代理配置 / 自动检测

### 🔑 机器码管理
- 查看 / 复制 / 重置
- 支持 Windows / macOS / Linux

### 🖥️ IDE 集成
- 检测 Kiro IDE 运行状态
- 一键启动 / 关闭
- 自动同步代理和模型设置

## 📸 截图

![首页](screenshots/首页.webp)
![账号管理](screenshots/账号管理.webp)
![桌面授权](screenshots/桌面授权.webp)
![规则管理](screenshots/规则管理.webp)
![设置](screenshots/设置.png)
![关于](screenshots/关于.png)

## 📥 下载

🌐 **[官网](https://vercel-api-lemon-five.vercel.app)** | 👉 **[GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - 推荐，双击安装
- macOS `.dmg` - 拖入 Applications
- Linux `.deb` / `.AppImage` - Debian/Ubuntu 或通用

## 💻 系统要求

- **Windows**: Windows 10/11 (64-bit)，需要 WebView2 (Win11 已内置)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon 通用)
- **Linux**: x86_64，需要 WebKitGTK

## 💬 交流反馈

- 🐛 [提交 Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ 群：[1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## ❓ 常见问题

**Q: 切换账号时提示 "bearer token invalid"**

A: Token 过期了，切换前先点「刷新」按钮。这是 Kiro 服务端返回的错误，不是管理器的问题。

**Q: 刷新 Token 失败**

A: 网络超时，手动再刷新一次或换个网络试试。

## 📝 源码说明

本仓库不再提供源码，仅用于发布 Release 和展示项目信息。

**⚠️ 本项目永久免费！如果有人向你收费，你被骗了！**

## 💖 赞助

如果这个项目对你有帮助，可以请作者喝杯咖啡 ☕

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="微信" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="支付宝" width="200">
</p>

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=hj01857655/kiro-account-manager&type=Date)](https://star-history.com/#hj01857655/kiro-account-manager&Date)

## 📄 许可证

[GPL-3.0](LICENSE)

## ⚠️ 免责声明

本软件仅供学习交流使用，请勿用于商业用途。使用本软件所产生的任何后果由用户自行承担。

---

<p align="center">Made with ❤️ by hj01857655</p>
