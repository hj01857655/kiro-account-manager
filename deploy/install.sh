#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v curl >/dev/null 2>&1; then
  apt-get update
  apt-get install -y ca-certificates curl
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker was not found; installing the official Docker Engine packages..."
  curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
  sh /tmp/get-docker.sh
  rm -f /tmp/get-docker.sh
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required." >&2
  exit 1
fi

if [[ ! -f .env ]]; then
  cp .env.example .env
  chmod 600 .env
  echo "Created .env. Set all replace-with-* values, then run this script again." >&2
  exit 2
fi

if grep -q 'replace-with-' .env; then
  echo ".env still contains placeholder secrets. Deployment stopped." >&2
  exit 2
fi

install -d -m 0750 data
if [[ "$(id -u)" -eq 0 ]]; then
  chown -R 10001:10001 data
else
  sudo chown -R 10001:10001 data
fi

docker compose build --pull
docker compose up -d
docker compose ps
echo "Kiro frontend: http://127.0.0.1:8080"
echo "Kiro backend health: http://127.0.0.1:3001/api/health"
