# Kiro Account Manager

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Version&color=green" alt="Version">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="Downloads">
  <img src="https://img.shields.io/badge/License-CC%20BY--NC--SA%204.0-orange" alt="License">
  <img src="https://img.shields.io/badge/Telegram-Канал-2CA5E0?logo=telegram" alt="Telegram Канал">
  <img src="https://img.shields.io/badge/Telegram-Сообщество-2CA5E0?logo=telegram" alt="Telegram Сообщество">
  <img src="https://img.shields.io/badge/Languages-Chinese%20%7C%20English%20%7C%20Russian-brightgreen" alt="Languages">
</p>

<p align="center">
  <b>🚀 Умное управление аккаунтами Kiro IDE - переключение одним кликом, мониторинг квоты</b>
</p>

<p align="center">
  🌐 <b><a href="https://kiro-website-six.vercel.app">Официальный сайт</a></b> | 
  📥 <b><a href="#-download">Скачать сейчас</a></b> | 
  💬 <b><a href="https://t.me/ide520">Telegram Сообщество</a></b>
</p>

> **📢 Поддержка языков**: Этот проект поддерживает интерфейсы на **китайском (упрощённом), английском и русском** языках.

---

## 🏗️ Обзор проекта

Kiro Account Manager - это десктопное приложение на базе **Tauri 2.x** для централизованного управления аккаунтами **Kiro IDE** и локальными конфигурациями.

**Технологический стек**: React 18 + Vite + shadcn/ui + TailwindCSS 4 | Rust + Tauri 2.x | Windows / macOS / Linux

**Основные модули**:
- Управление аккаунтами: импорт, экспорт, обновление, проверка, группировка, теги, удаление удалённо
- Аутентификация входа: Google / GitHub Social OAuth; AWS IAM Identity Center (BuilderId / Enterprise); Microsoft Entra ID / Azure AD (`external_idp` — только импорт + обновление токена, без интерактивного входа Microsoft в приложении)
- Интеграция Kiro: переключение аккаунтов, синхронизация моделей / прокси / MCP / Steering / Skills / Hooks / Custom Agents / Powers
- Автоматизация: автообновление токенов, автопереключение при низком балансе, привязка и сброс ID машины
- Возможности десктопа: Deep Link OAuth обратный вызов, одиночный экземпляр, системный трей, автообновление
- Возможности шлюза: встроенный Kiro API Gateway, поддерживает Anthropic Messages, OpenAI Responses, Chat Completions и потоковую пересылку; оценка токенов OpenAI через tiktoken

---

## 📥 Скачать

**Последняя версия v1.9.2** (выпущена 2026-06-17): пожалуйста, посетите [Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest) (автоматически поддерживается актуальной)

> Ссылки для скачивания ниже могут отставать, ориентируйтесь на Releases для последних версий.

| Платформа | Архитектура | Формат файла | Ссылка для скачивания |
|----------|-------------|-------------|---------------|
| 🪟 **Windows** | x64 | MSI установщик | [KiroAccountManager_1.9.2_x64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_x64_zh-CN.msi) |
| 🪟 **Windows** | ARM64 | MSI установщик | [KiroAccountManager_1.9.2_arm64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64_zh-CN.msi) |
| 🍎 **macOS** | x64 / Intel | DMG образ | [KiroAccountManager_1.9.2_x64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_x64.dmg) |
| 🍎 **macOS** | x64 / Intel | App архив | [KiroAccountManager_x64.app.tar.gz](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_x64.app.tar.gz) |
| 🍎 **macOS** | ARM64 / Apple Silicon (M1/M2/M3/M4/M5) | DMG образ | [KiroAccountManager_1.9.2_aarch64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_aarch64.dmg) |
| 🍎 **macOS** | ARM64 / Apple Silicon (M1/M2/M3/M4/M5) | App архив | [KiroAccountManager_aarch64.app.tar.gz](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_aarch64.app.tar.gz) |
| 🐧 **Linux** | x86_64 | AppImage | [KiroAccountManager_1.9.2_amd64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_amd64.AppImage) |
| 🐧 **Linux** | x86_64 | DEB пакет | [KiroAccountManager_1.9.2_amd64.deb](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_amd64.deb) |
| 🐧 **Linux** | x86_64 | RPM пакет | [KiroAccountManager-1.9.2-1.x86_64.rpm](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager-1.9.2-1.x86_64.rpm) |
| 🐧 **Linux** | ARM64 | AppImage | [KiroAccountManager_1.9.2_arm64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64.AppImage) |
| 🐧 **Linux** | ARM64 | DEB пакет | [KiroAccountManager_1.9.2_arm64.deb](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager_1.9.2_arm64.deb) |
| 🐧 **Linux** | ARM64 | RPM пакет | [KiroAccountManager-1.9.2-1.aarch64.rpm](https://github.com/hj01857655/kiro-account-manager/releases/download/v1.9.2/KiroAccountManager-1.9.2-1.aarch64.rpm) |

> **Примечание по стилю macOS**: Если возникают проблемы с отображением стиля, пожалуйста, настройте на основе исходного кода текущего репозитория (у меня нет устройства macOS, не могу воспроизвести и отладить).

**Системные требования**:
- **Windows**: Windows 10/11 (64-bit), требуется [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (встроен в Win11)
- **macOS**: macOS 10.15+ (Catalina и выше)
- **Linux**: архитектура x86_64 / ARM64, требуется WebKitGTK 4.0+

**Инструкции по установке**:
- **Windows**: дважды щёлкните файл `.msi` для установки
- **macOS**: откройте `.dmg` и перетащите в Applications. Приложение не подписано сертификатом Apple, поэтому при первом запуске macOS может сообщить, что оно «повреждено» или «от неизвестного разработчика». Откройте **Системные настройки → Конфиденциальность и безопасность**, найдите соответствующее сообщение внизу и нажмите **«Открыть всё равно»**; либо выполните `xattr -cr /Applications/KiroAccountManager.app` и снова откройте приложение. Для Mac на Apple Silicon (серия M) используйте пакет `aarch64`, для Mac на Intel — `x64`"Безопасность и конфиденциальность" при первом запуске
- **Linux AppImage**: запустите напрямую после `chmod +x`
- **Linux DEB**: установите с помощью `sudo dpkg -i`
- **Linux RPM**: установите с помощью `sudo rpm -i` или пакетного менеджера вашего дистрибутива

---

## 📝 История изменений

Записи сгруппированы по фактическим окнам публикации GitHub Release.  
**«Не выпущено»** — изменения уже в ветке `public`, но **ещё не** в GitHub Release (установочные пакеты по-прежнему по Releases).

### 🚧 Не выпущено (public, после v1.9.2)

- **Новое**: аккаунты Microsoft Entra ID / Azure AD (`external_idp`) — импорт + обновление токена через Microsoft OAuth2 token endpoint (не AWS OIDC); только импорт, без входа Microsoft в приложении
- **Исправлено**: при импорте Enterprise IdC извлекаются `startUrl` / provider из JWT `clientSecret`, предпочтение enterprise-доменам — меньше ложных BuilderId и 400 при refresh
- **Исправлено**: `getUsageLimits` больше не передаёт `profileArn` (у enterprise давало 400); Enterprise и обычные аккаунты на одном пути, region с приоритетом аккаунта и fallback `us-east-1` / `eu-central-1`
- **Усилено**: защита от injection в имени ветки Skill, подтверждение elevation restart, восстановление после poison lock deep link OAuth
- **Шлюз**: серии GPT-5.6 и Claude Sonnet 5; оценка OpenAI-токенов через tiktoken-rs (`o200k_base`); удалены устаревшие алиасы Claude 3
- **Шлюз**: удалено управление API Key — ключ клиента фиксированный `kiro` (принимаются `Authorization: Bearer kiro` и `x-api-key: kiro`); старые поля ключей в конфигурации читаются только для совместимости
- **Совместимость**: поддержка локального формата конфигурации Kiro IDE 1.0 — Hooks поддерживают новый формат `.json` (один файл может содержать несколько hook; поля изменены на `trigger` / `action`) и пользовательские глобальные hooks (`~/.kiro/hooks`); Custom Agents поддерживают новый JSON-формат (вместо Markdown front-matter). Старые файлы `.kiro.hook` и `.md` остаются читаемыми и редактируемыми
- **Совместимость**: управление правами перенесено с устаревших Trusted Commands на модель возможностей `permissions.yaml` Kiro IDE 1.0 — добавлен глобальный редактор правил доступа (capability / effect / match / exclude / policies); бэкенд напрямую читает/пишет `~/.kiro/settings/permissions.yaml`; устаревшая конфигурация `kiroAgent.*` мигрируется однократно при первом чтении, весь мёртвый код Trusted Commands удалён

### 🏗️ v1.9.2 - 2026-06-17 — поддержка архитектуры Linux ARM64 и исправление запуска macOS

> Этот релиз добавляет полную поддержку архитектуры **Linux ARM64** (Raspberry Pi, ARM-серверы и другие устройства), исправляет проблему запуска на macOS, когда окно не появлялось, и приводит User-Agent API-запросов в полное соответствие с реальным Kiro IDE.

#### 🏗️ Поддержка архитектуры
- **Новое**: полная поддержка сборки Linux ARM64 (aarch64) — доступны форматы AppImage, DEB и RPM
- **Исправлено**: обновление среды сборки macOS Intel (x86_64)

#### 🖥️ Совместимость macOS
- **Исправлено**: главное окно не отображается после запуска на macOS — добавлен 3-секундный резервный механизм: если фронтенд не вызывает `show_main_window` вовремя, окно показывается принудительно. Это устраняет ситуацию «процесс запущен, но окно не видно».

#### 🔒 Выравнивание UA
- **Исправлено**: User-Agent API-запросов полностью соответствует реальному Kiro IDE — management API (`getUsageLimits`, `ListAvailableModels`) использует `aws-sdk-js/1.0.0` + `codewhispererruntime#1.0.0` + `m/N,E`; streaming API (`generateAssistantResponse`) использует `aws-sdk-js/1.0.39` + `codewhispererstreaming#1.0.39` + `m/N`.

### 🚧 Не выпущено — изоляция аккаунтов и надёжность Kiro2API

> Фокус: проблемы долгого использования нескольких аккаунтов — machine ID на аккаунт, прокси на аккаунт, безопасное сохранение файлов, проброс ошибок upstream, совместимость Responses и Linux WebKit software rendering.

- **Новое**: `machineId` сохраняется на уровне аккаунта — импорт, онлайн-вход и нормализация генерируют стабильный случайный machine ID для аккаунтов без значения, вместо временного использования текущего системного machine ID.
- **Исправлено**: ручное и автоматическое переключение записывает machine ID целевого аккаунта — устраняет ситуацию, когда аккаунт уже изменён, но Kiro IDE state или request headers продолжают использовать старый / системный machine ID.
- **Изменено**: удалены legacy global machine ID settings.
- **Новое**: прокси для отдельного аккаунта — выбранный аккаунт может использовать собственный outbound proxy для Kiro2API / Kiro API без изменения Kiro IDE, Kiro CLI и системного прокси.
- **Исправлено**: корректная обработка profileArn ([#132](https://github.com/hj01857655/kiro-account-manager/issues/132)) — устранена ошибка 403 "User is not authorized to make this call", вызванная тем, что Social-аккаунтам (Google/GitHub) ошибочно присваивался BuilderId-default profileArn; BuilderId-аккаунты получают fallback, чтобы отсутствие profileArn больше не приводило к сбоям Kiro API.
- **Изменено**: сохраняется только последняя `.bak` копия — больше не создаются `accounts.backup-*.json` при каждом обновлении аккаунтов в AppData.
- **Исправлено**: восстановление из backup при отсутствии или ошибке замены `accounts.json` — прерванное сохранение больше не должно превращать список аккаунтов в пустое состояние.
- **Новое**: проброс Anthropic 429 — клиент видит настоящий upstream rate-limit response вместо generic wrapped failure.
- **Исправлено**: форма ответа OpenAI Responses для `/v1/responses` — меньше ошибок parsing из-за неполных output fields.
- **Новое**: ограничение потоков Linux WebKit software rendering — снижает длительную нагрузку WebKitWebProcess CPU в окружениях без GPU, на remote desktop и серверах.
- **Новое**: Linux ARM64 release builds — pipeline теперь также собирает AppImage / DEB / RPM на ARM64 Linux runner и формирует DEB / RPM updater metadata из фактических platform entries.

### 🛠️ v1.9.1 - 2026-06-02 — tool calls, Responses, request logs и восстановление квоты

> Фокус: совместимость Kiro2API — tool results, Responses output, не-200 passthrough, структурированные логи и восстановление квоты.

- **Исправлено**: tool results OpenAI Chat Completions больше не double-serialized — клиенты не получают JSON-looking string вместо ожидаемого объекта результата инструмента.
- **Исправлено**: запросы с отсутствующим или пустым `messages[].content` принимаются, что соответствует форматам tool-call / assistant-message у части сторонних клиентов.
- **Изменено**: tool results упорядочиваются по previous tool-use relationships, уменьшая mismatch при последовательных или параллельных вызовах нескольких tools.
- **Исправлено**: output и events для `/v1/responses`, уменьшая missing fields в Responses clients.
- **Новое**: raw JSON passthrough для auth / rate-limit / model errors.
- **Новое**: structured request logs — account, model, Region, status code, duration, streaming state и error summary записываются, чтобы сбой можно было связать с конкретным аккаунтом / моделью / Region.
- **Новое**: аккаунты автоматически включаются после восстановления квоты: capped или temporarily unavailable аккаунты возвращаются в pool после синхронизации доступной квоты.

### 🔄 v1.9.0 - 2026-06-01 — порядок переключения Kiro IDE и CLI logout

> Исправляет порядок записи при switching/logout, чтобы Kiro IDE и kiro-cli не сохраняли stale или partially updated token state.

- **Исправлено**: порядок записи файлов switch/logout согласован с Kiro IDE.
- **Исправлено**: logout и switch gates разделены.
- **Исправлено**: CLI logout очищает старые token и повторный logout.
- **Изменено**: usage probing охватывает все backend-supported Regions.
- **Изменено**: китайская терминология входа унифицирована.

### 🔐 v1.8.9 - 2026-06-01 — login callback, profileArn, auto-switching и release signing

> Исправляет login callback, поля кэша Kiro IDE, overage auto-switching, Region alignment, UTF-8 truncation и проверки release artifacts.

- **Исправлено**: AWS SSO использует loopback `redirect_uri` без порта.
- **Исправлено**: Social `expiresAt` и BuilderId `profileArn` записываются в формате Kiro IDE.
- **Новое**: отдельное действие logout в списке аккаунтов.
- **Исправлено**: auto-switching допускает capped accounts with overage headroom.
- **Исправлено**: kiro-cli обновляет token перед switching и очищает old keys.
- **Исправлено**: UTF-8 truncation, Region alignment и wildcard connection host.
- **Новое**: поддержка Claude Opus 4.8.
- **Исправлено**: provider identity в available-model cache.
- **Новое**: auto-update signing validation и MSI artifact selection fix.

### 🌍 v1.8.8 - 2026-05-31 — Bun, i18n и account status detection

> Улучшает скорость сборки, добавляет English/Russian UI и унифицирует detection статусов аккаунтов.

- **Изменено**: build workflow переведён на Bun, npm lockfile удалён.
- **Исправлено**: TOCTOU symlink риск token files; CSP и HTTP permissions ужесточены.
- **Новое**: detection suspended / banned / invalid / capped / overage.
- **Новое**: unusable accounts автоматически отключаются для auto-switching и Kiro2API routing.
- **Новое**: English/Russian UI и language switcher.
- **Изменено**: close-to-tray выключен по умолчанию.
- **Исправлено**: streaming `tool_use` восстанавливает original MCP tool names и missing tool-use start events.
- **Исправлено**: Enterprise gateway accounts больше не отправляют incompatible profileArn.

### 🚀 v1.8.7 - 2026-05-20 — core Kiro2API и account pool

> Большое расширение Kiro2API: OpenAI / Anthropic protocols, Prompt Cache, request logs, account-pool routing, API Keys, model mapping, prompt filters и Claude Code / Codex quick setup.

- **Новое**: Anthropic `/v1/messages`, OpenAI `/v1/chat/completions`, OpenAI `/v1/responses`.
- **Новое**: images, thinking parameters, tool calls, Responses session recovery и model mapping.
- **Исправлено**: Chat Completions streaming `completion_id` / `role`, Responses tool inheritance и Kiro API 400 cases.
- **Новое**: Prompt Cache mapping, simulator, payload size control, message trimming и token control.
- **Новое**: request logs, request/model/endpoint stats, log directory, search, filters, log levels и virtualized lists — запросы Kiro2API теперь можно разбирать, а не воспринимать как black box.
- **Новое**: account pool routing, route testing, API Key management, model mapping rules, prompt filters, Claude Code / Codex quick configuration — клиенты можно подключать без ручной сборки URL, ключей и model aliases.
- **Новое**: enabled/disabled state, overage controls, overage cap, quota auto-disable / auto-enable — аккаунты с доступным overage не смешиваются с полностью исчерпанными.
- **Изменено**: token auto-refresh перенесён в backend background tasks, чтобы обновление не зависело от открытой страницы.
- **Новое**: Windows ARM64 builds.
- **Удалено**: early MITM experiment и deprecated `/messages` route.
- **Исправлено**: client registration path traversal и backend security issues.

### ⚙️ v1.8.6 - 2026-05-10 — Responses foundation, account pool и IDE integration

> Закладывает основу Responses, переводит gateway accounts на account manager pool и улучшает IDE path, refresh before switching и machine ID backfill.

- **Новое**: OpenAI Responses API foundation.
- **Новое**: gateway account source defaults to account manager pool.
- **Новое**: account failure tracking, auto-disable, Balanced strategy, pool status view.
- **Новое**: Prompt Caching, token limits, payload size control, virtualized logs, search optimization.
- **Исправлено**: early Kiro API 400 и q.us-east-1 compatibility.
- **Новое**: custom Kiro IDE path, refresh token before switching, machine ID generation, current-account logout, context menus, app data directory, IDE Session Manager.
- **Исправлено**: `kiro://` deep links, FilterDropdown clipping, WiX template, auto-update public key, macOS DMG, multi-platform builds.

### 🧩 v1.8.5 - 2026-04-27 — login callback и Kiro upstream requests

> Исправляет online login callback, `kiro://` protocol registration и Kiro upstream headers, вызывавшие 403.

- **Исправлено**: AuthCallback close после успешного online login.
- **Исправлено**: `kiro://` указывает на текущий running app.
- **Исправлено**: missing Host header для q.us-east-1.
- **Исправлено**: удалён `TokenType: EXTERNAL_IDP`, вызывавший 403.
- **Улучшено**: account card spacing и window event handling.

Более ранние версии см. в GitHub Releases.

---

## 📸 Скриншоты

![Главная](screenshots/首页.png)
![Управление аккаунтами](screenshots/账号管理.png)
![Онлайн‑вход](screenshots/在线登录.png)
![Управление правилами](screenshots/规则管理.png)
![Управление сессиями](screenshots/会话管理.png)
![Прокси API Kiro](screenshots/Kiro2API.png)
![Настройки](screenshots/设置.png)
![О программе](screenshots/关于.png)

---

## ✨ Основные функции

### 🔐 Аутентификация входа
- **Social вход**: Google / GitHub OAuth, автоматическое обновление токена
- **IdC вход**: BuilderId / Enterprise, полный поток SSO OIDC
- **External IdP (Microsoft Entra ID / Azure AD)**: импорт существующих учётных данных и обновление токена (**не** «почта Outlook = Microsoft IdP»; смотрите token endpoint / тип auth)
- Примечание: у `external_idp` **нет** экрана входа Microsoft в приложении — вход через Kiro / портал организации, затем импорт

### 📊 Управление аккаунтами
- Двойной вид карточка / список, индикатор прогресса квоты, индикаторы типа подписки
- Обнаружение бана, обратный отсчёт истечения токена, выделение статуса
- Теги и группы, расширенная фильтрация (тип подписки / статус / степень использования)
- Enterprise usage: `getUsageLimits` без `profileArn`; приоритет region аккаунта и fallback по распространённым region

### 🔄 Переключение аккаунтов одним кликом
- Бесшовное переключение аккаунтов Kiro IDE, автоматический сброс ID машины
- Автоматический пропуск заблокированных аккаунтов, автопереключение при низком балансе
- Автоматическое включение аккаунтов при восстановлении квоты

### 📦 Пакетные операции
- Импорт / экспорт JSON, импорт из Kiro IDE / kiro-cli (включая классификацию `external_idp`)
- Пакетное обновление / удаление / присвоение тегов / удалённый выход

### 🔌 Синхронизация конфигурации Kiro
Управление всё в одном: серверы MCP, правила Steering, Hooks, Skills, Custom Agents, Powers

### ⚙️ Системные настройки
Четыре темы, блокировка модели ИИ, автономный режим агента, автообновление токена, конфигурация прокси

### 🌐 Kiro API шлюз
Встроенный шлюз, совместимый с OpenAI, поддерживает прямую интеграцию со сторонними инструментами, такими как Cursor / Continue / Cline.
- Совместим с Anthropic `/v1/messages`, OpenAI `/v1/responses`, `/v1/chat/completions`
- Модели: Claude 4.x / Sonnet 5, серии GPT-5.6 (см. `/v1/models` шлюза и список в UI)
- Оценка токенов OpenAI-совместимых запросов через tiktoken (`o200k_base`)
- Интеллектуальное понижение модели, балансировка нагрузки нескольких аккаунтов, аутентификация по фиксированному ключу (`kiro`)
- Проброс исходного формата JSON для не-200 ответов
- Проброс ответов об ошибках Anthropic 429
- Оптимизация структуры тела ответа Responses
- Упорядочивание результатов инструментов и улучшение отслеживания StreamInfo

**Подключение клиентов (фиксированный ключ `kiro`)**: шлюз по умолчанию слушает `http://127.0.0.1:8765` (актуальный адрес см. на странице шлюза). Ключ клиента фиксированный — `kiro`, создание и управление ключами в приложении недоступно:

```text
Base URL: http://127.0.0.1:8765
API Key:  kiro
```

Принимаются оба стиля заголовка: `Authorization: Bearer kiro` (стиль OpenAI) и `x-api-key: kiro` (стиль Anthropic). Быстрые конфигурации Claude Code / Codex, формируемые на странице шлюза, можно скопировать напрямую.

---

## ❓ Часто задаваемые вопросы

**Q: Ошибка "bearer token invalid" при переключении аккаунтов**
A: Токен истёк, нажмите кнопку "Обновить" перед переключением.

**Q: macOS показывает "приложение повреждено и не может быть открыто"**
A: Выполните `xattr -cr /Applications/KiroAccountManager.app` и откройте снова.

**Q: Приложение не выходит после нажатия кнопки закрытия?**
A: Оно скрыто в системный трей, нажмите "Выйти из приложения" в меню трея для полного выхода.

**Q: Windows MSI показывает "установлена та же версия"**
A: Продолжайте установку (v1.8.3+ поддерживает обновление с перезаписью).

---

## 📝 Сборка из исходников

```bash
git clone https://github.com/hj01857655/kiro-account-manager.git
cd kiro-account-manager
bun install
bun run tauri dev    # Режим разработки
bun run tauri build  # Сборка релиза
```

Предварительные требования: Node.js 20+, цепочка инструментов Rust, системные зависимости WebView.

**⚠️ Этот проект навсегда бесплатен! Если кто-то взимает с вас плату, вас обманули!**

---

## 💬 Обратная связь

- 🐛 [Отправить Issue](https://github.com/hj01857655/kiro-account-manager/issues)
- 📢 Telegram Канал: [https://t.me/kiro520](https://t.me/kiro520)
- 💬 Telegram Сообщество: [https://t.me/ide520](https://t.me/ide520)

---

## 🤝 Спонсоры

<table>
  <tr>
    <td align="center" width="50%">
      <a href="https://fishxcode.com/" target="_blank"><b>🐟 FishXCode</b></a><br>
      <sub>Стабильная служба ретрансляции Claude API</sub>
    </td>
    <td align="center" width="50%">
      <a href="https://synai996.space/" target="_blank"><b>🤖 SynAI996</b></a><br>
      <sub>Высокопроизводительная платформа прокси API моделей ИИ</sub>
    </td>
  </tr>
</table>

## 💖 Спонсорство

Если этот проект помог вам, вы можете угостить автора кофе ☕ (пожалуйста, укажите ваше имя пользователя GitHub для удобного добавления в список спонсоров)

<p align="center">
  <img src="src/assets/donate/wechat.jpg" alt="WeChat" width="200">
  <img src="src/assets/donate/alipay.jpg" alt="Alipay" width="200">
</p>

Спасибо спонсорам: 🌟 [shiro123444](https://github.com/shiro123444)

---

## ⭐ История звёзд

[![Star History Chart](https://api.star-history.com/svg?repos=hj01857655/kiro-account-manager&type=Date)](https://star-history.com/#hj01857655/kiro-account-manager&Date)

---

## 📄 Лицензия

[CC BY-NC-SA 4.0](LICENSE) - **Коммерческое использование запрещено**

Это программное обеспечение предназначено только для обучения и общения. Пользователи несут ответственность за любые последствия, возникающие в результате использования этого программного обеспечения.

---

<p align="center">Сделано с ❤️ by hj01857655</p>
<p align="center"><sub>Последнее обновление: 2026-06-17 | Версия: v1.9.2</sub></p>