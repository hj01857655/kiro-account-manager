PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS admins (
    username TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#64748b',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    email TEXT,
    label TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    provider TEXT,
    user_id TEXT,
    auth_method TEXT,
    expires_at TEXT,
    region TEXT,
    profile_arn TEXT,
    group_id TEXT REFERENCES groups(id) ON DELETE SET NULL,
    machine_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    proxy_json TEXT,
    usage_json TEXT,
    models_json TEXT,
    secret_blob TEXT NOT NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    disabled_reason TEXT,
    last_failure_at TEXT,
    last_refreshed_at TEXT,
    last_checked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS account_tags (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    linked_at TEXT NOT NULL,
    PRIMARY KEY (account_id, tag_id)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    action TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    message TEXT NOT NULL,
    client_ip TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gateway_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint TEXT NOT NULL,
    model TEXT,
    account_id TEXT,
    status_code INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_status ON accounts(status, enabled);
CREATE INDEX IF NOT EXISTS idx_accounts_group ON accounts(group_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_gateway_metrics_created ON gateway_metrics(created_at DESC);
