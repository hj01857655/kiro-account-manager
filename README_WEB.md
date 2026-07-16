# Kiro Account Manager — Web edition

This repository contains a Linux-deployable web management panel derived from the original Tauri application. It runs as two containers behind host Nginx:

```text
Browser -> Nginx TLS :443 -> frontend :8080
                         -> /api backend :3001 -> SQLite /data/kiro.db
                         -> /v1 Gateway :3001 -> Kiro APIs
```

The web runtime never reads or modifies a server-side Kiro IDE installation. The original desktop source remains in the repository for traceability but is excluded from Docker images. See [MIGRATION_ANALYSIS.md](MIGRATION_ANALYSIS.md) for the module audit.

## Implemented web features

- React 18/Vite responsive login, dashboard, account list/detail/editor/import, groups, Gateway settings, system settings, and audit logs.
- Axum/Tokio REST server on `127.0.0.1:3001` by default.
- SQLite/sqlx migrations, parameterized queries, WAL mode, and `/data/kiro.db` by default.
- account import/list/edit/delete/export, groups/tags, token refresh, Kiro usage/model queries, machine IDs, account checks, per-account proxy, and Gateway auto-switching.
- administrator JWT in an HttpOnly/Secure/SameSite=Lax cookie, Argon2id password verification, CSRF token, login throttling, API throttling, and failed-login audit records.
- AES-256-GCM encryption for access/refresh/ID tokens, client secrets, passwords, and proxy passwords.
- independent Gateway API key, basic Anthropic/OpenAI/Responses compatibility, and incremental SSE generated from Kiro AWS EventStream frames.
- request body limit, same-origin default, optional exact CORS origin, trusted proxy headers only from a loopback peer, safe error envelopes, request IDs, and structured JSON logs.

Account export is deliberately redacted. It never returns complete tokens. Moving secrets between installations should be done by importing the original account source into the destination, not by producing an unencrypted browser download.

## API response format

Management API success:

```json
{ "success": true, "data": {} }
```

Management API error:

```json
{ "success": false, "error": { "code": "bad_request", "message": "..." } }
```

Routes:

- `POST /api/auth/login`, `GET /api/auth/me`, `POST /api/auth/logout`
- `GET|POST /api/accounts`, `GET|PUT|DELETE /api/accounts/{id}`
- `POST /api/accounts/import`, `GET /api/accounts/export`
- `POST /api/accounts/{id}/refresh`, `POST /api/accounts/{id}/check`
- `GET /api/accounts/{id}/usage`, `GET /api/accounts/{id}/models`
- `GET|POST /api/groups`, `PUT|DELETE /api/groups/{id}`
- `GET|POST /api/tags`
- `GET|PUT /api/settings`, `GET /api/logs`, `GET /api/dashboard`
- public health check: `GET /api/health`
- Gateway: `POST /v1/messages`, `POST /v1/chat/completions`, `POST /v1/responses`, `GET /v1/models`
- User panel: `POST /api/user/auth/register`, `POST /api/user/auth/login`, `GET /api/user/auth/me`, `POST /api/user/auth/logout`, `GET /api/user/models`, `POST /api/user/chat`

All management routes except login and health require the administrator cookie. All unsafe management requests also require the session CSRF header. Gateway routes accept only `Authorization: Bearer <GATEWAY_API_KEY>` and do not accept the administrator session.

## Environment variables

| Variable | Required | Default | Description |
|---|---:|---|---|
| `ADMIN_USERNAME` | yes | — | Administrator username bootstrapped at startup. |
| `ADMIN_PASSWORD` | yes | — | Administrator password, minimum 12 characters; stored as an Argon2id hash. Changing it and restarting rotates the login password. |
| `JWT_SECRET` | yes | — | HMAC secret for 8-hour administrator sessions, minimum 32 characters. Rotating it invalidates sessions. |
| `DATA_ENCRYPTION_KEY` | yes | — | Exactly 32 random bytes encoded as base64. Never rotate without decrypting/re-encrypting existing account rows. Losing it makes stored secrets unrecoverable. |
| `BIND_ADDR` | no | `127.0.0.1:3001` | Backend listener. Compose sets `0.0.0.0:3001` inside the isolated container and publishes it only on host loopback. |
| `DATABASE_PATH` | no | `/data/kiro.db` | SQLite file path. Migrations run automatically. |
| `COOKIE_SECURE` | no | `true` | Adds `Secure` to the admin cookie. Keep `true` with HTTPS; set `false` only for direct local HTTP development. |
| `ALLOWED_ORIGIN` | no | empty | Optional exact browser origin for a separate frontend. Empty means no cross-origin CORS responses, which is the recommended same-origin deployment. |
| `TRUSTED_PROXY_IPS` | no | `127.0.0.1,::1` | Comma-separated exact peer IPs allowed to supply `X-Forwarded-For`/`X-Real-IP`. Compose overrides this with its pinned bridge gateway. Do not use a wildcard or public subnet. |
| `KIRO_DOCKER_SUBNET` | no | `172.29.0.0/24` | Private Compose bridge subnet; change it if it conflicts with an existing server network. |
| `KIRO_DOCKER_GATEWAY` | no | `172.29.0.1` | Compose bridge gateway and the only container peer trusted for host-Nginx forwarded headers. |
| `GATEWAY_ENABLED` | no | `false` | Enables `/v1/*`. |
| `GATEWAY_API_KEY` | when enabled | — | Independent Gateway bearer secret, minimum 32 characters. |
| `GATEWAY_DEFAULT_ACCOUNT` | no | empty | Exact account UUID. Empty selects the best active account. |
| `GATEWAY_AUTO_SWITCH` | no | `true` | Retries with another active account after authentication, balance, rate-limit, suspension, or upstream-server errors. |
| `USER_REGISTRATION_ENABLED` | no | `true` | Allows new email/password registrations for the public user panel. |
| `RUST_LOG` | no | application info | Rust `tracing` filter. Logs are structured JSON and exclude request credentials/bodies. |

Generate secure values once:

```bash
openssl rand -hex 32       # JWT_SECRET
openssl rand -base64 32    # DATA_ENCRYPTION_KEY
openssl rand -hex 32       # GATEWAY_API_KEY
openssl rand -base64 24    # ADMIN_PASSWORD
```

## Deployment on Ubuntu/Debian

```bash
# Copy this completed web project to the server, or clone the branch containing it.
sudo install -d -o "$USER" -g "$USER" /opt/kiro-account-manager-web
cd /opt/kiro-account-manager-web

cp .env.example .env
chmod 600 .env
nano .env

chmod +x deploy/install.sh
sudo ./deploy/install.sh
curl --fail http://127.0.0.1:3001/api/health
```

Or run Compose directly after creating `.env` and a writable data directory:

```bash
sudo install -d -o 10001 -g 10001 -m 0750 data
docker compose build --pull
docker compose up -d
docker compose ps
docker compose logs --tail=100 kiro-backend
```

The Compose ports are bound to `127.0.0.1`; they are not publicly reachable without Nginx.

## Nginx and Certbot for kiro.nvdx.de

First point the DNS A/AAAA record to the server. Obtain the certificate before installing the final TLS configuration so Nginx does not reference missing certificate files:

```bash
sudo apt update
sudo apt install -y nginx certbot
sudo systemctl stop nginx
sudo certbot certonly --standalone -d kiro.nvdx.de \
  --agree-tos --no-eff-email -m YOUR_EMAIL

sudo cp deploy/nginx.conf /etc/nginx/sites-available/kiro.nvdx.de
sudo ln -sfn /etc/nginx/sites-available/kiro.nvdx.de /etc/nginx/sites-enabled/kiro.nvdx.de
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl enable --now nginx
```

Test renewal:

```bash
sudo certbot renew --dry-run
```

The supplied config routes `/` to the frontend and `/api/` and `/v1/` to the backend, forwards client IP/protocol, supports WebSocket upgrades, caps uploads at 10 MB, disables proxy buffering for streams, and adds TLS/security headers.

## Local development

Backend prerequisites are a current stable Rust toolchain and SQLite build dependencies:

```bash
cp .env.example .env
# For local HTTP only, set COOKIE_SECURE=false and DATABASE_PATH=./data/kiro.db.
cd backend
cargo run
```

In another terminal:

```bash
cd frontend
npm ci
npm run dev
```

Vite proxies `/api` and `/v1` to `127.0.0.1:3001`.

## Verification

```bash
cd frontend && npm ci && npm run build
cd ../backend && cargo test && cargo clippy --all-targets -- -D warnings
cd .. && docker compose config
docker compose up -d --build
curl --fail http://127.0.0.1:3001/api/health
```

## Backup and upgrade

Stop writes before copying SQLite and always preserve the encryption key separately:

```bash
docker compose stop kiro-backend
cp -a data/kiro.db "data/kiro.db.$(date +%F-%H%M%S).bak"
docker compose start kiro-backend
```

Upgrade:

```bash
git pull --ff-only
docker compose build --pull
docker compose up -d
```

Database migrations are applied automatically and are forward-only. Back up both `data/kiro.db` and the securely stored `DATA_ENCRYPTION_KEY` before upgrading.
