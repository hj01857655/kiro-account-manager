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
  <b>🚀 Smart Kiro IDE account management with one-click switching and quota monitoring</b>
</p>

---

## ✨ Features

### 🔐 Account Login
- **Desktop OAuth** - Desktop authorization for Google/GitHub/BuilderId
- **Web Portal OAuth** - Web authorization in WebView window
- Two methods complement each other for reliable login

### 📊 Account Display
- Card grid layout, clear at a glance
- Quota progress bar (main/trial/bonus)
- Subscription type badge (Free/PRO/PRO+)
- Token expiration countdown
- Status highlight (normal/expired/banned/current)

### 🔄 One-Click Switch
- Seamless Kiro IDE account switching
- Auto reset machine ID
- Real-time switch progress

### 📦 Batch Operations
- Batch refresh / batch delete
- JSON import/export (Social & IdC formats)
- SSO Token batch import
- Keyword search filter

### 🔌 Kiro Config
- **MCP Servers** - CRUD, enable/disable
- **Powers** - View, install, uninstall
- **Steering Rules** - View, edit

### ⚙️ System Settings
- Four themes (light/dark/purple/green)
- AI model selection & lock
- Auto token refresh (configurable interval)
- Auto reset machine ID on switch

### 🌐 Browser & Proxy
- Custom browser / auto detect
- Incognito mode launch
- HTTP proxy config / auto detect

### 🔑 Machine Code
- View / backup / restore / reset
- Windows / macOS support

### 🖥️ IDE Integration
- Detect Kiro IDE running status
- One-click start / stop
- Auto sync proxy and model settings

## 📸 Screenshots

| Home | Account Management |
|:---:|:---:|
| ![Home](screenshots/首页.png) | ![Accounts](screenshots/账号管理.png) |

| Login | Settings |
|:---:|:---:|
| ![Login](screenshots/登录页.png) | ![Settings](screenshots/设置.png) |

## 📥 Download

[![Release](https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?style=flat-square)](https://github.com/hj01857655/kiro-account-manager/releases/latest)

👉 **[Download Latest Version](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

| Platform | File Type | Description |
|----------|-----------|-------------|
| Windows | `.msi` | Recommended, double-click to install |
| Windows | `.exe` | NSIS installer |
| macOS | `.dmg` | Drag to Applications |

## 💻 System Requirements

- **Windows**: Windows 10/11 (64-bit), WebView2 required (built-in on Win11)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon universal)

## 🛠️ Tech Stack

- **Frontend**: React 18 + Vite 5 + TailwindCSS 3 + Lingui (i18n)
- **Backend**: Tauri 2.x + Rust + Tokio
- **Icons**: Lucide React
- **Storage**: Local JSON files

## 🚀 Quick Start

1. Download the installer for your platform from [Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)
2. Install and launch the application
3. Login with Google, GitHub, or BuilderId
4. Manage your Kiro accounts with ease!

## 💬 Feedback

- 🐛 [Submit Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ Group: [1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=hj01857655/kiro-account-manager&type=Date)](https://star-history.com/#hj01857655/kiro-account-manager&Date)

## ⚠️ Disclaimer

This software is for learning and communication purposes only. Do not use for commercial purposes. Users are responsible for any consequences.

## 📄 License

[GPL-3.0](LICENSE) - Modifications must be open-sourced.

---

<p align="center">Made with ❤️ by hj01857655</p>
<p align="center">If this project helps you, please give it a ⭐!</p>
