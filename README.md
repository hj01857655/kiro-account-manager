# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/github/license/hj01857655/kiro-account-manager?color=orange&t=20241224" alt="License">
  <img src="https://img.shields.io/badge/QQ群-1020204332-12B7F5?logo=tencentqq" alt="QQ群">
</p>

<p align="center">
  <a href="#english">English</a> | <a href="#简体中文">简体中文</a> | <a href="#русский">Русский</a>
</p>

---

# 简体中文

<p align="center">
  <b>🚀 智能管理 Kiro IDE 账号，一键切换，配额监控</b>
</p>

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
- JSON 导入导出（Social / IdC 格式）
- SSO Token 批量导入
- 关键词搜索过滤

### 🔌 Kiro 配置
- **MCP 服务器** - 增删改查、启用/禁用
- **Steering 规则** - 查看、编辑

### ⚙️ 系统设置
- 四种主题（浅色/深色/紫色/绿色）
- AI 模型选择与锁定
- Token 自动刷新（可配置间隔）
- 切号自动重置机器 ID

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

� **[点击这里下载最新版本](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - 推荐，双击安装
- Windows `.exe` - NSIS 安装程序
- macOS `.dmg` - 拖入 Applications

## 💻 系统要求

- **Windows**: Windows 10/11 (64-bit)，需要 WebView2 (Win11 已内置)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon 通用)

## � 交流反求馈

- 🐛 [提交 Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ 群：[1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## �  源码说明

**后端源码（Rust）已从本仓库移除。**

有人违反 GPL-3.0 协议，将本项目用于商业用途——**对这款免费软件收费出售**——且未开源修改后的代码。这明显违反了 GPL-3.0 的要求：

- ✅ 衍生作品必须以 GPL-3.0 协议开源
- ✅ 必须提供或公开源代码
- ✅ 必须保留许可证和版权声明

**⚠️ 本项目永久免费！如果有人向你收费，你被骗了！**

## 💖 赞助

如果这个项目对你有帮助，可以请作者喝杯咖啡 ☕

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="微信" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="支付宝" width="200">
</p>

---

# English

<p align="center">
  <b>🚀 Smart Kiro IDE account management with one-click switching and quota monitoring</b>
</p>

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

### � Kiro Config
- **MCP Servers** - CRUD, enable/disable
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

👉 **[Download Latest Version](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - Recommended, double-click to install
- Windows `.exe` - NSIS installer
- macOS `.dmg` - Drag to Applications

## 💻 System Requirements

- **Windows**: Windows 10/11 (64-bit), WebView2 required (built-in on Win11)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon universal)

## 💬 Feedback

- 🐛 [Submit Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ Group: [1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## 🚫 Source Code Notice

**The backend source code (Rust) has been removed from this repository.**

Some individuals violated the GPL-3.0 license by using this project for commercial purposes - **charging users for this free software** - without open-sourcing their modifications. This is a clear violation of the GPL-3.0 terms.

**⚠️ This project is and will always be FREE! If anyone charges you, you have been scammed!**

## 💖 Sponsor

If this project helps you, consider buying me a coffee ☕

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="WeChat" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="Alipay" width="200">
</p>

---

# Русский

<p align="center">
  <b>🚀 Умное управление аккаунтами Kiro IDE с переключением в один клик и мониторингом квот</b>
</p>

## ✨ Возможности

### 🔐 Вход в аккаунт
- **Desktop OAuth** - Авторизация на рабочем столе для Google/GitHub/BuilderId
- **Web Portal OAuth** - Веб-авторизация в окне WebView
- Два метода дополняют друг друга для надёжного входа

### 📊 Отображение аккаунтов
- Карточный интерфейс, всё наглядно
- Прогресс-бар квот (основные/пробные/бонусные)
- Значок типа подписки (Free/PRO/PRO+)
- Обратный отсчёт истечения токена
- Подсветка статуса (активен/истёк/заблокирован/текущий)

### 🔄 Переключение в один клик
- Бесшовное переключение аккаунта Kiro IDE
- Автоматический сброс Machine ID
- Отображение прогресса в реальном времени

### 📦 Пакетные операции
- Массовое обновление / удаление
- Импорт/экспорт JSON (Social / IdC форматы)
- Массовый импорт SSO Token
- Поиск по ключевым словам

### 🔌 Конфигурация Kiro
- **MCP серверы** - CRUD, включение/отключение
- **Steering правила** - Просмотр, редактирование

### ⚙️ Системные настройки
- Четыре темы (светлая/тёмная/фиолетовая/зелёная)
- Выбор и блокировка AI модели
- Автообновление токенов (настраиваемый интервал)
- Автосброс Machine ID при переключении

### 🌐 Браузер и прокси
- Пользовательский браузер / автоопределение
- Запуск в режиме инкогнито
- Настройка HTTP прокси / автоопределение

### 🔑 Управление Machine Code
- Просмотр / резервное копирование / восстановление / сброс
- Поддержка Windows / macOS

### 🖥️ Интеграция с IDE
- Определение статуса работы Kiro IDE
- Запуск / остановка в один клик
- Автосинхронизация прокси и настроек модели

## 📸 Скриншоты

| Главная | Управление аккаунтами |
|:---:|:---:|
| ![Главная](screenshots/首页.png) | ![Аккаунты](screenshots/账号管理.png) |

| Вход | Настройки |
|:---:|:---:|
| ![Вход](screenshots/登录页.png) | ![Настройки](screenshots/设置.png) |

## 📥 Скачать

👉 **[Скачать последнюю версию](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - Рекомендуется, двойной клик для установки
- Windows `.exe` - NSIS установщик
- macOS `.dmg` - Перетащите в Applications

## 💻 Системные требования

- **Windows**: Windows 10/11 (64-bit), требуется WebView2 (встроен в Win11)
- **macOS**: macOS 10.15+ (универсальный для Intel/Apple Silicon)

## 💬 Обратная связь

- 🐛 [Создать Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 Telegram: [@kiro520](https://t.me/kiro520)

## 🚫 Примечание об исходном коде

**Исходный код бэкенда (Rust) был удалён из этого репозитория.**

Некоторые лица нарушили лицензию GPL-3.0, используя этот проект в коммерческих целях — **взимая плату за это бесплатное ПО** — без открытия исходного кода своих модификаций. Это явное нарушение требований GPL-3.0:

- ✅ Производные работы должны быть открыты под GPL-3.0
- ✅ Исходный код должен быть предоставлен или доступен
- ✅ Лицензия и уведомления об авторских правах должны быть сохранены

**⚠️ Этот проект всегда будет БЕСПЛАТНЫМ! Если кто-то берёт с вас деньги, вас обманули! Пожалуйста, сообщайте о таких нарушениях.**

## 💖 Поддержать

Если проект вам помог, можете угостить автора кофе ☕

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="WeChat" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="Alipay" width="200">
</p>

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=hj01857655/kiro-account-manager&type=Date)](https://star-history.com/#hj01857655/kiro-account-manager&Date)

## 📄 License / 开源协议 / Лицензия

[GPL-3.0](LICENSE)

## ⚠️ Disclaimer / 免责声明 / Отказ от ответственности

本软件仅供学习交流使用，请勿用于商业用途。使用本软件所产生的任何后果由用户自行承担。

This software is for learning and communication purposes only. Users are responsible for any consequences.

---

<p align="center">Made with ❤️ by hj01857655</p>
