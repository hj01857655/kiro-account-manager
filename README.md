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

🌐 **[官网](https://vercel-lajuwps1g-hj01857655s-projects-fa88a766.vercel.app)** | 👉 **[GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - 推荐，双击安装
- macOS `.dmg` - 拖入 Applications
- Linux `.deb` / `.AppImage` - Debian/Ubuntu 或通用

## 💻 系统要求

- **Windows**: Windows 10/11 (64-bit)，需要 WebView2 (Win11 已内置)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon 通用)
- **Linux**: x86_64，需要 WebKitGTK

## � 交流反求馈

- 🐛 [提交 Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ 群：[1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## ❓ 常见问题

**Q: 切换账号时提示 "bearer token invalid"**

A: Token 过期了，切换前先点「刷新」按钮。这是 Kiro 服务端返回的错误，不是管理器的问题。

**Q: 刷新 Token 失败**

A: 网络超时，手动再刷新一次或换个网络试试。

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

### 📊 Account Display
- 🆕 Card/List dual view toggle
- Quota progress bar (main/trial/bonus)
- Subscription type badge (Free/PRO/PRO+)
- Token expiration countdown
- Status highlight (normal/expired/banned/current)

### 🔄 One-Click Switch
- Seamless Kiro IDE account switching
- Auto reset machine ID
- Real-time switch progress

### 📦 Batch Operations
- 🆕 Batch refresh / batch delete / batch tagging
- JSON import/export (Social & IdC formats)
- SSO Token batch import
- Keyword search filter

### 🏷️ Tag Management 🆕
- Custom tags (name/color)
- Batch tag assignment
- Filter accounts by tag

### 🔍 Advanced Filter & Sort 🆕
- Filter by subscription (Free/PRO/PRO+)
- Filter by status (normal/banned)
- Sort by usage/added time/trial expiry
- Tri-state sorting (desc→asc→cancel)

### 🔌 Kiro Config
- **MCP Servers** - CRUD, enable/disable
- **Steering Rules** - View, edit

### ⚡ KiroGate
- Built-in OpenAI compatible API proxy
- Independent token management
- Support multiple AI clients

### ⚙️ System Settings
- Four themes (light/dark/purple/green)
- AI model selection & lock
- 🆕 Codebase indexing toggle
- 🆕 Trust all commands toggle
- Auto token refresh (configurable interval)
- Auto reset machine ID on switch
- 🆕 Privacy mode (hide sensitive info)

### 🌐 Browser & Proxy
- Custom browser / auto detect
- Incognito mode launch
- HTTP proxy config / auto detect

### 🔑 Machine Code
- View / copy / reset
- Windows / macOS / Linux support

### 🖥️ IDE Integration
- Detect Kiro IDE running status
- One-click start / stop
- Auto sync proxy and model settings

## 📸 Screenshots

![Home](screenshots/首页.webp)
![Accounts](screenshots/账号管理.webp)
![Desktop Auth](screenshots/桌面授权.webp)
![Rules](screenshots/规则管理.webp)
![Settings](screenshots/设置.png)
![About](screenshots/关于.png)

## 📥 Download

🌐 **[Website](https://vercel-lajuwps1g-hj01857655s-projects-fa88a766.vercel.app)** | 👉 **[GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - Recommended, double-click to install
- macOS `.dmg` - Drag to Applications
- Linux `.deb` / `.AppImage` - Debian/Ubuntu or universal

## 💻 System Requirements

- **Windows**: Windows 10/11 (64-bit), WebView2 required (built-in on Win11)
- **macOS**: macOS 10.15+ (Intel/Apple Silicon universal)
- **Linux**: x86_64, WebKitGTK required

## 💬 Feedback

- 🐛 [Submit Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ Group: [1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

## ❓ FAQ

**Q: "bearer token invalid" error when switching accounts**

A: Token expired. Click "Refresh" button before switching. This error comes from Kiro server, not the manager.

**Q: Token refresh failed**

A: Network issue or account restricted by Kiro. Try re-login.

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

### 📊 Отображение аккаунтов
- Переключение между карточками и списком
- Прогресс-бар квот (основные/пробные/бонусные)
- Значок типа подписки (Free/PRO/PRO+)
- Обратный отсчёт истечения токена
- Подсветка статуса (активен/истёк/заблокирован/текущий)

### 🔄 Переключение в один клик
- Бесшовное переключение аккаунта Kiro IDE
- Автоматический сброс Machine ID
- Отображение прогресса в реальном времени

### 📦 Пакетные операции
- Массовое обновление / удаление / присвоение тегов
- Импорт/экспорт JSON (Social / IdC форматы)
- Массовый импорт SSO Token
- Поиск по ключевым словам

### 🏷️ Управление тегами
- Пользовательские теги (название/цвет)
- Массовое присвоение тегов
- Фильтрация аккаунтов по тегам

### 🔍 Расширенная фильтрация и сортировка
- Фильтр по подписке (Free/PRO/PRO+)
- Фильтр по статусу (активен/заблокирован)
- Сортировка по использованию/дате добавления/истечению пробного периода
- Трёхрежимная сортировка (убыв→возр→отмена)

### 🔌 Конфигурация Kiro
- **MCP серверы** - CRUD, включение/отключение
- **Steering правила** - Просмотр, редактирование

### ⚡ KiroGate
- Встроенный OpenAI-совместимый API прокси
- Независимое управление токенами
- Поддержка различных AI клиентов

### ⚙️ Системные настройки
- Четыре темы (светлая/тёмная/фиолетовая/зелёная)
- Выбор и блокировка AI модели
- 🆕 Переключатель индексации кодовой базы
- 🆕 Переключатель доверия всем командам
- Автообновление токенов (настраиваемый интервал)
- Автосброс Machine ID при переключении
- 🆕 Режим конфиденциальности (скрытие данных)

### 🌐 Браузер и прокси
- Пользовательский браузер / автоопределение
- Запуск в режиме инкогнито
- Настройка HTTP прокси / автоопределение

### 🔑 Управление Machine Code
- Просмотр / копирование / сброс
- Поддержка Windows / macOS / Linux

### 🖥️ Интеграция с IDE
- Определение статуса работы Kiro IDE
- Запуск / остановка в один клик
- Автосинхронизация прокси и настроек модели

## 📸 Скриншоты

![Главная](screenshots/首页.webp)
![Аккаунты](screenshots/账号管理.webp)
![Авторизация](screenshots/桌面授权.webp)
![Правила](screenshots/规则管理.webp)
![Настройки](screenshots/设置.png)
![О программе](screenshots/关于.png)

## 📥 Скачать

🌐 **[Сайт](https://vercel-lajuwps1g-hj01857655s-projects-fa88a766.vercel.app)** | 👉 **[GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)**

- Windows `.msi` - Рекомендуется, двойной клик для установки
- macOS `.dmg` - Перетащите в Applications
- Linux `.deb` / `.AppImage` - Debian/Ubuntu или универсальный

## 💻 Системные требования

- **Windows**: Windows 10/11 (64-bit), требуется WebView2 (встроен в Win11)
- **macOS**: macOS 10.15+ (универсальный для Intel/Apple Silicon)
- **Linux**: x86_64, требуется WebKitGTK

## 💬 Обратная связь

- 🐛 [Создать Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 🛒 [Магазин](https://pay.ldxp.cn/shop/U60F42WD)

## ❓ Частые вопросы

**Q: Ошибка "bearer token invalid" при переключении аккаунта**

A: Токен истёк. Нажмите «Обновить» перед переключением. Это ошибка сервера Kiro, а не менеджера.

**Q: Не удалось обновить токен**

A: Проблема сети или аккаунт ограничен Kiro. Попробуйте войти заново.

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
