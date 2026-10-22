# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-orange" alt="License">
  <img src="https://img.shields.io/badge/Telegram-Channel-2CA5E0?logo=telegram" alt="Telegram Channel">
  <img src="https://img.shields.io/badge/Telegram-Community-2CA5E0?logo=telegram" alt="Telegram Community">
  <img src="https://img.shields.io/badge/Languages-Chinese%20%7C%20English%20%7C%20Russian-brightgreen" alt="Languages">
</p>

<p align="center">
  <b>🚀 Smart Kiro IDE Account Management - One-click Switching, Quota Monitoring</b>
</p>

<p align="center">
  🌐 <b><a href="https://kiro-website-six.vercel.app">Official Website</a></b> | 
  📥 <b><a href="#-download">Download Now</a></b> | 
  💬 <b><a href="https://t.me/ide520">Telegram Community</a></b>
</p>

> **📢 Language Support**: This project supports **Chinese (Simplified), English, and Russian** interfaces.

---

## 🏗️ Project Overview

Kiro Account Manager is a desktop application based on **Tauri 2.x** for centralized management of **Kiro IDE** accounts and local configurations.

**Tech Stack**: React 18 + Vite + shadcn/ui + TailwindCSS 4 | Rust + Tauri 2.x | Windows / macOS / Linux

**Core Modules**:
- Account Management: Import, export, refresh, verify, grouping, tagging, remote deletion
- Login Authentication: Google / GitHub Social OAuth; AWS IAM Identity Center (BuilderId / Enterprise); Microsoft Entra ID / Azure AD (`external_idp` — import + token refresh only, no in-app Microsoft login)
- Kiro Integration: Switch accounts, sync models / proxy / MCP / Steering / Skills / Hooks / Custom Agents / Powers
- Automation: Auto-refresh tokens, auto-switch on low balance, machine ID binding and reset
- Desktop Capabilities: Deep Link OAuth callback, single instance, system tray, auto-update
- Gateway Capabilities: Built-in Kiro API Gateway, supports Anthropic Messages, OpenAI Responses, Chat Completions and streaming; OpenAI-side token estimates via tiktoken

---

## 📥 Download

**Latest Version v1.9.2** (Released 2026-06-17): Please visit [Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest) (auto-kept up-to-date)

> The download links below may lag behind, refer to Releases for the latest versions.

| Platform | Architecture | File Format | Download Link |
|----------|-------------|-------------|---------------|
| 🪟 **Windows** | x64 | MSI Installer | [KiroAccountManager_1.9.2_x64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_x64_zh-CN.msi) |
| 🪟 **Windows** | ARM64 | MSI Installer | [KiroAccountManager_1.9.2_arm64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64_zh-CN.msi) |
| 🍎 **macOS** | x64 / Intel | DMG Image | [KiroAccountManager_1.9.2_x64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_x64.dmg) |
| 🍎 **macOS** | x64 / Intel | App Archive | [KiroAccountManager_x64.app.tar.gz](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_x64.app.tar.gz) |
| 🍎 **macOS** | ARM64 / Apple Silicon (M1/M2/M3/M4/M5) | DMG Image | [KiroAccountManager_1.9.2_aarch64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_aarch64.dmg) |
| 🍎 **macOS** | ARM64 / Apple Silicon (M1/M2/M3/M4/M5) | App Archive | [KiroAccountManager_aarch64.app.tar.gz](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_aarch64.app.tar.gz) |
| 🐧 **Linux** | x86_64 | AppImage | [KiroAccountManager_1.9.2_amd64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_amd64.AppImage) |
| 🐧 **Linux** | x86_64 | DEB Package | [KiroAccountManager_1.9.2_amd64.deb](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_amd64.deb) |
| 🐧 **Linux** | x86_64 | RPM Package | [KiroAccountManager-1.9.2-1.x86_64.rpm](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager-1.9.2-1.x86_64.rpm) |
| 🐧 **Linux** | ARM64 | AppImage | [KiroAccountManager_1.9.2_arm64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64.AppImage) |
| 🐧 **Linux** | ARM64 | DEB Package | [KiroAccountManager_1.9.2_arm64.deb](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64.deb) |
| 🐧 **Linux** | ARM64 | RPM Package | [KiroAccountManager-1.9.2-1.aarch64.rpm](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager-1.9.2-1.aarch64.rpm) |

> **macOS Style Note**: If style display issues occur, please adjust based on the current repository source code (I don't have a macOS device, cannot reproduce and debug).

**System Requirements**:
- **Windows**: Windows 10/11 (64-bit), requires [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (built-in on Win11)
- **macOS**: macOS 10.15+ (Catalina and above)
- **Linux**: x86_64 / ARM64 architecture, requires WebKitGTK 4.0+

**Installation Instructions**:
- **Windows**: Double-click `.msi` file to install
- **macOS**: Open the `.dmg` and drag to Applications. The app is not signed with an Apple certificate, so on first launch macOS may report it as "damaged" or "from an unidentified developer". Open **System Settings → Privacy & Security**, find the prompt near the bottom and click **Open Anyway**; alternatively run `xattr -cr /Applications/KiroAccountManager.app` and relaunch. Apple Silicon (M-series) and Intel Macs use the `aarch64` and `x64` packages respectively—pick the one matching your chip
- **Linux AppImage**: Run directly after `chmod +x`
- **Linux DEB**: Install with `sudo dpkg -i`
- **Linux RPM**: Install with `sudo rpm -i` or the package manager for your distribution

---

## 📝 Changelog

Entries are grouped by the actual GitHub Release publish windows.  
**Unreleased** lists changes already on the `public` branch that are **not** yet in a GitHub Release (installed packages still follow Releases).

### 🚧 Unreleased (public, since v1.9.2)

- **New**: Microsoft Entra ID / Azure AD accounts (`external_idp`) — import + token refresh via Microsoft OAuth2 token endpoint (not AWS OIDC); import-only, no in-app Microsoft login
- **Fix**: Enterprise IdC import extracts `startUrl` / provider from `clientSecret` JWT and prefers enterprise domains to avoid mis-classifying as BuilderId (refresh 400)
- **Fix**: `getUsageLimits` no longer sends `profileArn` (enterprise 400); Enterprise usage uses the same path as other accounts with region preference + `us-east-1` / `eu-central-1` fallback
- **Harden**: Skill branch-name injection guard, elevation restart confirmation, OAuth deep-link lock poison recovery
- **Gateway**: GPT-5.6 series and Claude Sonnet 5; OpenAI token estimates via tiktoken-rs (`o200k_base`); removed retired Claude 3 alias mapping
- **Gateway**: Removed API Key management — the client key is fixed to `kiro` (both `Authorization: Bearer kiro` and `x-api-key: kiro` are accepted); legacy key fields in existing configs are read for compatibility only
- **Compat**: Follow Kiro IDE 1.0 local config format — Hooks support the new `.json` format (one file may hold multiple hooks; fields changed to `trigger` / `action`) plus user-level global hooks (`~/.kiro/hooks`); Custom Agents support the new JSON format (replacing Markdown front-matter). Legacy `.kiro.hook` and `.md` files remain readable and editable
- **Compat**: Permission management migrated from the legacy Trusted Commands to Kiro IDE 1.0's `permissions.yaml` capability model — added a global permission rule editor (capability / effect / match / exclude / policies); the backend reads/writes `~/.kiro/settings/permissions.yaml` directly; legacy `kiroAgent.*` config is migrated as a one-time fallback on first read, and all legacy Trusted Commands dead code was removed

### 🏗️ v1.9.2 - 2026-06-17 — Linux ARM64 Architecture Support and macOS Startup Fix

> This release adds full **Linux ARM64** architecture support (Raspberry Pi, ARM servers, etc.), fixes the macOS startup issue where the window did not appear, and aligns API request User-Agent strings with the real Kiro IDE.

#### 🏗️ Architecture Support
- **New**: Linux ARM64 (aarch64) full build support — provides AppImage, DEB, and RPM formats
- **Fix**: macOS Intel (x86_64) build environment update

#### 🖥️ macOS Compatibility
- **Fix**: Main window not showing after launch on macOS — added a 3-second fallback that force-shows the window if the frontend doesn't trigger `show_main_window` in time, preventing the "process running but no window" issue.

#### 🔒 UA Alignment
- **Fix**: API request User-Agent fully aligned with the real Kiro IDE — management APIs (`getUsageLimits`, `ListAvailableModels`) use `aws-sdk-js/1.0.0` + `codewhispererruntime#1.0.0` + `m/N,E`; streaming API (`generateAssistantResponse`) uses `aws-sdk-js/1.0.39` + `codewhispererstreaming#1.0.39` + `m/N`.

### 🚧 Unreleased — Account Isolation and Kiro2API Reliability

> Focuses on long-running multi-account issues: account-scoped machine IDs, per-account proxies, safer account-file saves, upstream error passthrough, Responses compatibility, and Linux WebKit software-rendering behavior.

#### 🔑 Account Machine ID Isolation
- **New**: Account-scoped `machineId` persistence — imported accounts, online login, and normalization now generate a stable random machine ID for accounts that do not already have one, instead of borrowing the current system machine ID at switch time.
- **Fix**: Manual switching and auto-switching write the target account's own machine ID — fixes the case where the account changes but Kiro IDE state or request headers still use the previous/current system machine ID.
- **Change**: Removed legacy global machine ID compatibility settings; switching now uses the account's own `machineId`.

#### 🌐 Per-account Proxy and BuilderId
- **New**: Per-account proxy configuration — a specific account can use its own outbound proxy for Kiro2API / Kiro API calls without changing Kiro IDE, Kiro CLI, or the system proxy.
- **Fix**: profileArn handling ([#132](https://github.com/hj01857655/kiro-account-manager/issues/132)) — fixes the 403 "User is not authorized to make this call" error caused by Social accounts (Google/GitHub) being incorrectly assigned the BuilderId default profileArn; BuilderId accounts also gain a fallback so missing profileArn no longer fails Kiro API requests.
- **Improve**: Account edit layout for groups, tags, proxy, and machine ID fields.

#### 💾 Account Files and Kiro2API
- **Change**: Account saves keep only the latest `.bak` backup — avoids continuously creating `accounts.backup-*.json` files under AppData on every account update.
- **Fix**: Restore from backup when `accounts.json` is missing or replacement fails — prevents an interrupted save from turning the account list into an empty state.
- **New**: Anthropic 429 raw error passthrough — callers now see the real upstream rate-limit response instead of a generic wrapped failure.
- **Fix**: Preserve upstream JSON for non-200 responses where possible — authentication, rate-limit, and model errors keep their actionable fields for clients and logs.
- **Fix**: OpenAI Responses body shape for `/v1/responses` — reduces client parsing failures caused by incomplete output fields.
- **Change**: Move MCP configuration out of proxy settings.
- **New**: Linux WebKit software-rendering thread limit — reduces long-running WebKitWebProcess CPU saturation on GPU-less, remote desktop, and server environments.
- **New**: Linux ARM64 release builds — the release pipeline now also builds AppImage / DEB / RPM artifacts on an ARM64 Linux runner and derives DEB / RPM updater metadata from the actual platform entries.

### 🛠️ v1.9.1 - 2026-06-02 — Tool Calls, Responses, Request Logs, and Quota Recovery

> Focuses on Kiro2API protocol compatibility: Chat Completions tool results, Responses output shape, non-200 passthrough, structured logs, and quota recovery.

#### 🔧 Chat Completions Tool Calls
- **Fix**: Tool results are no longer double-serialized — Chat Completions clients no longer receive a JSON-looking string that cannot be parsed as the expected tool result object.
- **Fix**: Requests with missing or empty `messages[].content` are accepted, matching common tool-call / assistant-message shapes produced by third-party clients.
- **Change**: Tool results are ordered by previous tool-use relationships, reducing mismatches when multiple tools are invoked in sequence or concurrently.

#### 📡 Responses, Errors, and Logs
- **Fix**: `/v1/responses` output and event fields are completed, reducing missing fields in Responses clients.
- **New**: Raw JSON passthrough for authentication, rate-limit, and model errors.
- **New**: Structured request logs — account, model, Region, status code, duration, streaming state, and error summary are recorded so failures can be traced to the exact account/model/Region.
- **New**: Accounts are re-enabled automatically after quota recovery, so capped or temporarily unavailable accounts return to the pool after usable quota is synchronized.

### 🔄 v1.9.0 - 2026-06-01 — Kiro IDE Switch Ordering and CLI Logout

> Fixes account switching and logout write order so Kiro IDE and kiro-cli do not keep stale or partially updated token state.

- **Fix**: Account file write ordering for switch/logout now matches Kiro IDE behavior.
- **Fix**: Logout and switch gates are separated.
- **Fix**: CLI logout clears old tokens and handles repeated logout states.
- **Change**: Usage probing covers all backend-supported Regions.
- **Change**: Chinese authentication terminology is unified.

### 🔐 v1.8.9 - 2026-06-01 — Login Callback, profileArn, Auto-switching, and Release Signing

> Fixes login callback compatibility, Kiro IDE cache fields, overage auto-switching, Region alignment, UTF-8 truncation, and release artifact checks.

- **Fix**: AWS SSO uses loopback `redirect_uri` without a port.
- **Fix**: Social `expiresAt` and BuilderId `profileArn` are written in Kiro IDE-compatible form.
- **New**: Explicit logout action in the account list.
- **Fix**: Auto-switching allows capped accounts with overage headroom.
- **Fix**: kiro-cli switching refreshes tokens and cleans old keys.
- **Fix**: UTF-8 truncation, Region alignment, and wildcard connection host generation.
- **New**: Claude Opus 4.8 model support.
- **Fix**: Available-model cache provider identity.
- **New**: Auto-update signing validation and MSI artifact selection fix.

### 🌍 v1.8.8 - 2026-05-31 — Bun, i18n, and Account Status Detection

> Improves build speed, adds English/Russian UI, and unifies account status detection across sync, refresh, usage, and model-list queries.

- **Change**: Build workflow migrated to Bun and npm lockfile removed.
- **Fix**: Token-file TOCTOU symlink risk; CSP and HTTP permissions tightened.
- **New**: suspended / banned / invalid / capped / overage status detection.
- **New**: Unusable accounts are automatically disabled for auto-switching and Kiro2API routing.
- **New**: English and Russian UI with a settings language switcher.
- **Change**: Close-to-tray is disabled by default.
- **Fix**: Streaming `tool_use` restores original MCP tool names and emits missing tool-use start events.
- **Fix**: Enterprise gateway accounts no longer send incompatible profileArn.

### 🚀 v1.8.7 - 2026-05-20 — Core Kiro2API and Account Pool Release

> Major Kiro2API expansion: OpenAI / Anthropic protocols, Prompt Cache, request logs, account-pool routing, API Keys, model mapping, prompt filters, and Claude Code / Codex quick setup.

- **New**: Anthropic `/v1/messages`, OpenAI `/v1/chat/completions`, and OpenAI `/v1/responses` compatibility.
- **New**: Image content, thinking parameters, tool calls, Responses session recovery, and model mapping.
- **Fix**: Chat Completions streaming `completion_id` / `role`, Responses tool inheritance, and multiple Kiro API 400 cases.
- **New**: Prompt Cache mapping, simulator, payload size control, message trimming, and token control.
- **New**: Request logs, request/model/endpoint stats, log directory access, search, filters, log levels, and virtualized lists — Kiro2API requests can now be inspected instead of treated as a black box.
- **New**: Account pool routing, route testing, API Key management, model mapping rules, prompt filters, and Claude Code / Codex quick configuration — clients can be connected without manually assembling URLs, keys, and model aliases.
- **New**: Account enabled/disabled state, overage controls, overage cap display, and quota-based auto-disable / auto-enable — usable overage accounts are no longer treated the same as exhausted accounts.
- **Change**: Token auto-refresh moved to backend background tasks so refresh behavior is not tied to whether the page is currently open.
- **New**: Windows ARM64 builds.
- **Remove**: Early MITM experiment and deprecated `/messages` route.
- **Fix**: Client registration path traversal and backend security issues.

### ⚙️ v1.8.6 - 2026-05-10 — Responses Foundation, Account Pool, and IDE Integration

> Establishes the Responses foundation, switches gateway accounts to the account manager pool, and improves Kiro IDE path detection, token refresh before switching, and machine ID backfill.

- **New**: OpenAI Responses API foundation.
- **New**: Gateway account source defaults to the account manager pool.
- **New**: Account failure tracking, auto-disable, Balanced strategy, and pool status view.
- **New**: Prompt Caching, token limits, payload size control, virtualized request logs, and search optimization.
- **Fix**: Early Kiro API 400 cases and q.us-east-1 compatibility.
- **New**: Custom Kiro IDE path, token refresh before switching, machine ID generation, current-account logout, context menus, app data directory entry, and IDE Session Manager.
- **Fix**: `kiro://` deep links, FilterDropdown clipping, WiX template, auto-update public key, macOS DMG, and multi-platform builds.

### 🧩 v1.8.5 - 2026-04-27 — Login Callback and Kiro Upstream Request Fixes

> Fixes online login callback behavior, `kiro://` protocol registration, and Kiro upstream headers that caused 403 responses.

- **Fix**: AuthCallback close behavior after successful online login.
- **Fix**: `kiro://` points to the currently running app.
- **Fix**: Missing Host header for q.us-east-1 upstream requests.
- **Fix**: Removed `TokenType: EXTERNAL_IDP` header that caused 403 responses.
- **Improve**: Account card spacing and window event handling.

For older versions, see GitHub Releases.

---

## 📸 Screenshots

![Home](screenshots/首页.png)
![Account Management](screenshots/账号管理.png)
![Online Login](screenshots/在线登录.png)
![Rules Management](screenshots/规则管理.png)
![Session Management](screenshots/会话管理.png)
![Kiro API Proxy](screenshots/Kiro2API.png)
![Settings](screenshots/设置.png)
![About](screenshots/关于.png)

---

## ✨ Core Features

### 🔐 Login Authentication
- **Social Login**: Google / GitHub OAuth, automatic token refresh
- **IdC Login**: BuilderId / Enterprise, complete SSO OIDC flow
- **External IdP (Microsoft Entra ID / Azure AD)**: import existing credentials and refresh tokens (not “Outlook email ⇒ Microsoft IdP”; depends on token endpoint / auth type)
- Note: `external_idp` has **no in-app Microsoft login UI** — sign in via Kiro / your org portal, then import

### 📊 Account Management
- Card / List dual view, quota progress bar, subscription type indicators
- Ban detection, token expiration countdown, status highlighting
- Tags and groups, advanced filtering (subscription type / status / usage rate)
- Enterprise usage: `getUsageLimits` without `profileArn`; prefer account region with common-region fallback

### 🔄 One-click Account Switching
- Seamless Kiro IDE account switching, automatic machine ID reset
- Auto-skip banned accounts, auto-switch on low balance
- Auto-enable accounts when quota is restored

### 📦 Batch Operations
- JSON import/export, import from Kiro IDE / kiro-cli (including `external_idp` classification)
- Batch refresh / delete / tag / remote logout

### 🔌 Kiro Configuration Sync
One-stop management: MCP servers, Steering rules, Hooks, Skills, Custom Agents, Powers

### ⚙️ System Settings
Four themes, AI model locking, Agent autonomous mode, auto token refresh, proxy configuration

### 🌐 Kiro API Gateway
Built-in OpenAI-compatible gateway, supports direct integration with third-party tools like Cursor / Continue / Cline.
- Compatible with Anthropic `/v1/messages`, OpenAI `/v1/responses`, `/v1/chat/completions`
- Models include Claude 4.x / Sonnet 5, GPT-5.6 series (see gateway `/v1/models` and UI list)
- OpenAI-compatible token estimation via tiktoken (`o200k_base`)
- Intelligent model degradation, multi-account load balancing, fixed-key authentication (`kiro`)
- Passthrough original JSON format for non-200 responses
- Anthropic 429 error response passthrough
- Responses format response body structure optimization
- Tool result ordering and StreamInfo tracking enhancement

**Client access (fixed key `kiro`)**: the gateway listens on `http://127.0.0.1:8765` by default (see the gateway page for the actual address). The client key is fixed to `kiro` — keys can no longer be created or managed in the app:

```text
Base URL: http://127.0.0.1:8765
API Key:  kiro
```

Both header styles are accepted: `Authorization: Bearer kiro` (OpenAI-style) and `x-api-key: kiro` (Anthropic-style). Claude Code / Codex quick configs generated on the gateway page can be copied directly.

---

## ❓ FAQ

**Q: "bearer token invalid" error when switching accounts**
A: Token has expired, click the "Refresh" button before switching.

**Q: macOS shows "app is damaged and can't be opened"**
A: Execute `xattr -cr /Applications/KiroAccountManager.app` and reopen.

**Q: Application doesn't exit after clicking close button?**
A: It's hidden to system tray, click "Exit App" in tray menu to completely exit.

**Q: Windows MSI shows "same version already installed"**
A: Continue installation (v1.8.3+ supports overwrite upgrade).

---

## 📝 Build from Source

```bash
git clone https://github.com/hj01857655/kiro-account-manager.git
cd kiro-account-manager
bun install
bun run tauri dev    # Development mode
bun run tauri build  # Build release
```

Prerequisites: Node.js 20+, Rust toolchain, system WebView dependencies.

**⚠️ This project is permanently free! If someone charges you, you've been scammed!**

---

## 💬 Feedback

- 🐛 [Submit Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 📢 Telegram Channel: [https://t.me/kiro520](https://t.me/kiro520)
- 💬 Telegram Community: [https://t.me/ide520](https://t.me/ide520)

---

## 🤝 Sponsors

<table>
  <tr>
    <td align="center" width="50%">
      <a href="https://fishxcode.com/" target="_blank"><b>🐟 FishXCode</b></a><br>
      <sub>Stable Claude API relay service</sub>
    </td>
    <td align="center" width="50%">
      <a href="https://synai996.space/" target="_blank"><b>🤖 SynAI996</b></a><br>
      <sub>High-performance AI model API proxy platform</sub>
    </td>
  </tr>
</table>

## 💖 Sponsorship

If this project helps you, you can buy the author a coffee ☕ (please note your GitHub username for easy addition to the sponsor list)

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="WeChat" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="Alipay" width="200">
</p>

Thanks to sponsors: 🌟 [shiro123444](https://github.com/shiro123444)

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=hj01857655/kiro-account-manager&type=Date)](https://star-history.com/#hj01857655/kiro-account-manager&Date)

---

## 📄 License

[CC BY-NC-SA 4.0](LICENSE) - **Commercial use prohibited**

This software is for learning and communication purposes only. Users are responsible for any consequences arising from the use of this software.

---

<p align="center">Made with ❤️ by hj01857655</p>
<p align="center"><sub>Last updated: 2026-06-17 | Version: v1.9.2</sub></p>