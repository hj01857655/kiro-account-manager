# Data Model: Local Kiro 2API Sidecar Integration

**Feature**: [Link to Spec](./spec.md)

## 1. Local Configuration (Client-Side)
Stored in `app-settings` or specific `local-server` store.

| Field | Type | Description | Default |
|---|---|---|---|
| `enabled` | `boolean` | Whether the server should auto-start on app launch. | `true` |
| `port` | `number` | The local port to listen on. | `7860` |
| `dataPath` | `string` | (Optional) Custom path for KV storage. | `<APP_DATA>/kiro-server` |

## 2. Server Status (Runtime)
In-memory state within the React application / Rust Backend.

```typescript
type ServerStatus =
  | 'STOPPED'   // Process is not running
  | 'STARTING'  // Process spawned, waiting for health check
  | 'RUNNING'   // Healthy and accepting requests
  | 'STOPPING'  // Graceful shutdown in progress
  | 'ERROR';    // Process crashed or failed to start
```

## 3. Log Entry (Runtime)
Ephemeral logs streamed from the sidecar process.

```typescript
interface LogEntry {
  id: string;        // UUID for React keys
  timestamp: number; // Unix timestamp
  source: 'stdout' | 'stderr';
  message: string;   // The raw log line
}
```

## 4. Server-Side Data (Deno KV)
The internal data structure used by `kiro2api-deno` (no changes, just reference).

*   `kiro_auth_tokens`: List of Anthropic API Keys + Usage stats.
*   `kiro_settings`: (If any) Server-side settings.
