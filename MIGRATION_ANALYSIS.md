# Web migration analysis

The original `public` branch was audited before implementation. The new web runtime lives in `frontend/` and `backend/`; the original `src/` and `src-tauri/` directories remain available as a reference and are not part of either deployment image.

## 1. Modules that can be reused

- React presentation language: account cards, dashboard statistics, theme tokens, navigation hierarchy, status badges, and the existing management workflow.
- Pure Rust account concepts: account/auth-method fields, status normalization, usage/quota interpretation, region/profile fields, per-account proxy behavior, and machine ID generation.
- Kiro HTTP protocol details: Management API URLs and headers, Social and AWS IdC token-refresh payloads, model-list calls, runtime endpoint selection, AWS EventStream framing, model aliases, and Gateway route semantics.
- Gateway concepts: independent API key, account-pool selection, failure accounting, automatic account switch, `/v1/models`, Anthropic Messages, OpenAI Chat Completions, and Responses streaming formats.

## 2. Modules strongly coupled to Tauri

- `src-tauri/src/main.rs`, Tauri application state, commands using `AppHandle`/`State`, and frontend calls using `invoke` or Tauri events.
- System tray, native window state, updater, deep links, desktop OAuth windows, elevated restart, desktop notifications, and WebView dialogs.
- Local Kiro IDE/CLI configuration, process discovery, local database/files, and account switching that mutates the workstation installation.
- Frontend uses of Tauri filesystem, dialog, shell, updater, process, window, and event plugins.

These modules are intentionally not compiled into the web images.

## 3. Refactored modules

- JSON/file account storage was replaced with parameterized `sqlx` queries and SQLite migrations.
- Tauri commands were replaced with Axum handlers and a uniform JSON envelope.
- sensitive account fields moved into an AES-256-GCM encrypted blob; public account DTOs expose only presence flags.
- desktop event/state access was replaced with shared Axum state, Tokio tasks, structured `tracing`, and HTTP/SSE responses.
- frontend data access was replaced with same-origin `fetch`, HttpOnly cookies, an in-memory CSRF token, and native browser file selection/download.
- the Gateway now selects encrypted database accounts and uses a completely separate bearer key.

## 4. Implementation order used

1. Isolate the original source and audit Tauri boundaries.
2. Add startup configuration checks, SQLite schema/migrations, encrypted storage, and administrator bootstrap.
3. Add login/session/logout, rate limits, CSRF, security headers, and audit logging.
4. Add account/group/tag/settings/log/dashboard REST APIs and Kiro refresh/usage/model/check operations.
5. Add Gateway authentication, account selection, auto-switching, Kiro Runtime calls, EventStream decoding, and SSE output.
6. Build the React web console without Tauri packages.
7. Add Docker, Nginx, installation, TLS, backup, and verification documentation.

## Compatibility boundary

The web Gateway implements text conversations and incremental SSE for all three requested public protocols. Tool definitions are included as guarded textual context. The original desktop Gateway's full image/document fetcher, signed thinking-history reconstruction, MCP bridge, and rich tool-call continuation remain desktop-specific reference code and are not exposed by the first web build. This avoids bringing local-file and desktop-state access into the server security boundary.
