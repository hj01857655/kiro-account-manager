# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-orange" alt="License">
</p>

<p align="center">
  <b>🚀 Smart Kiro IDE account management, one-click switch, quota monitoring</b>
</p>

<p align="center">
  🌐 <a href="README.md">简体中文</a> |
  English |
  <a href="README.tr.md">Türkçe</a>
</p>

---

## 📥 Download

**Latest Version**: [GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)

| Platform | Download |
|----------|----------|
| 🪟 **Windows** | [KiroAccountManager_x64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_x64_zh-CN.msi) |
| 🍎 **macOS (Intel)** | [KiroAccountManager_x64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_x64.dmg) |
| 🍎 **macOS (Apple Silicon)** | [KiroAccountManager_aarch64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_aarch64.dmg) |
| 🐧 **Linux (AppImage)** | [KiroAccountManager_amd64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_amd64.AppImage) |
| 🐧 **Linux (deb)** | [KiroAccountManager_amd64.deb](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_amd64.deb) |

**System Requirements**:
- **Windows**: Windows 10/11 (64-bit), WebView2 required (built-in on Win11)
- **macOS**: macOS 10.15+ (Intel / Apple Silicon universal)
- **Linux**: x86_64, WebKitGTK required

---

## ✨ Core Features

### 🔐 Online Login

- **Social Login** — Google / GitHub via desktop OAuth flow with auto token refresh
- **IdC Login** — AWS IAM Identity Center (Builder ID & Enterprise accounts), full SSO OIDC support

### 📊 Account Management

- Card view / Table view toggle
- Quota progress bars (main quota / trial / bonus)
- Subscription type badges (Free / PRO / PRO+)
- Token expiry countdown
- Status highlighting (active / expired / banned / current)
- Ban detection (423 Locked / 403 TEMPORARILY_SUSPENDED)

### 🔄 One-Click Account Switch

- Seamless Kiro IDE account switching
- Auto machine ID reset (random / bound mode)
- Real-time switch progress
- Auto-skip banned accounts

### 📦 Batch Operations

**Import & Export**
- JSON format (file import / paste)
- Import from Kiro IDE (auto-detect logged-in account)
- Import from kiro-cli (read SQLite database)
- Export to JSON (batch selection supported)

**Batch Management**
- Batch refresh (smart concurrency control)
- Batch delete / batch label
- Remote delete (unregister from AWS, Google/GitHub only)
- Keyword search & filter

### 🏷️ Labels & Groups

- Custom labels (name / color)
- Batch label assignment
- Account groups with group-based filtering

### 🔍 Advanced Filtering

- Filter by subscription type (Free / PRO / PRO+)
- Filter by status (active / banned)
- Sort by usage rate / added date / trial expiry
- Tri-state sorting (descending → ascending → none)

### 🔌 Kiro Configuration

- **MCP Servers** — CRUD, enable / disable
- **Steering Rules** — View, edit

### ⚙️ System Settings

- 4 themes (Light / Dark / Purple / Green)
- AI model selection & lock
- Auto token refresh (configurable interval)
- Auto machine ID reset on switch
- Privacy mode (email masking)
- Auto switch on low balance (configurable threshold)
- Custom browser / auto-detect, incognito mode
- HTTP proxy / system proxy auto-detect

### 🔑 Machine ID Management

- View / Copy / Reset
- Windows / macOS / Linux support

### 🖥️ IDE Integration

- Detect Kiro IDE running status
- One-click start / stop
- Auto-sync proxy and model settings

### ⚡ API Proxy Service

Need an OpenAI-compatible API? Use the standalone project **[kiro-gateway](https://github.com/hj01857655/kiro-gateway)**

---

## 📸 Screenshots

![Home](screenshots/首页.webp)
![Accounts](screenshots/账号管理.webp)
![Desktop OAuth](screenshots/桌面授权.webp)
![Rules](screenshots/规则管理.webp)
![Settings](screenshots/设置.png)
![About](screenshots/关于.png)

---

## 💬 Feedback

- 🐛 [Submit Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ Group: [1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

---

## ❓ FAQ

**Q: "bearer token invalid" when switching accounts?**
A: Token expired. Click "Refresh" before switching.

**Q: Token refresh failed?**
A: Network timeout. Try refreshing again or switch networks.

---

## 🔗 Related Projects

- **[kiro-gateway](https://github.com/hj01857655/kiro-gateway)** — Kiro API Gateway, OpenAI/Anthropic compatible interface

---

## 📄 License

[CC BY-NC-SA 4.0](LICENSE) — **Commercial use prohibited**

## ⚠️ Disclaimer

This software is for learning and communication purposes only. **Commercial use is strictly prohibited.** Users bear all responsibility for any consequences arising from the use of this software.

**⚠️ This project is permanently free! If someone charges you for it, you've been scammed!**

---

<p align="center">Made with ❤️ by hj01857655</p>