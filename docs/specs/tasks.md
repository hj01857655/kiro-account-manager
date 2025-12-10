# Tasks: Local Kiro 2API Sidecar Integration

**Feature**: Local Kiro 2API Sidecar Integration
**Spec**: [Link to Spec](./spec.md)
**Status**: ✅ Completed (2025-12-10)

## Phase 1: Setup ✅ 100%
**Goal**: Initialize project structure and migration for local server integration.
**Independent Test Criteria**: Deno code exists in `kiro-account-manager/server` and compiles to a binary.

- [x] T001 Create server directory structure in `kiro-account-manager/server`
- [x] T002 Copy `kiro2api-deno` source code to `kiro-account-manager/server`
- [x] T003 Update `kiro-account-manager/server/deno.json` with `compile` task for sidecar binary generation
- [x] T004 Create `kiro-account-manager/scripts/build-server.sh` helper script for cross-platform binary compilation and renaming
- [x] T005 Create `kiro-account-manager/src-tauri/bin` directory for storing compiled sidecar binaries
- [x] T006 Configure `kiro-account-manager/src-tauri/tauri.conf.json` to register `bin/kiro-server` as `externalBin`
- [x] T007 Configure `kiro-account-manager/src-tauri/capabilities/default.json` to allow shell execute permissions for the sidecar

## Phase 2: Foundational ✅ 100%
**Goal**: Core backend adaptations for local environment compatibility.
**Independent Test Criteria**: Server binary respects `KIRO_KV_PATH` and `PORT` environment variables.

- [x] T008 Modify `kiro-account-manager/server/auth/kv_store.ts` to use `KIRO_KV_PATH` environment variable for KV path if set
- [x] T009 Modify `kiro-account-manager/server/main.ts` to respect `PORT` environment variable and bind to localhost only by default
- [x] T010 Verify `kiro-account-manager/server/main.ts` handles static file serving correctly when bundled via `deno compile --include`

## Phase 3: User Story 1 - Start/Stop & Status ✅ 100%
**Goal**: Basic process lifecycle management from Tauri backend.
**Independent Test Criteria**: Tauri app can spawn the sidecar process and report its running status.

- [x] T011 Create `kiro-account-manager/src-tauri/src/server_process.rs` module structure
- [x] T012 Implement `start_local_server` command in `kiro-account-manager/src-tauri/src/server_process.rs` using `Command::new_sidecar`
- [x] T013 Implement `stop_local_server` command in `kiro-account-manager/src-tauri/src/server_process.rs`
- [x] T014 Implement `get_server_status` command in `kiro-account-manager/src-tauri/src/server_process.rs`
- [x] T015 Register new commands in `kiro-account-manager/src-tauri/src/main.rs`
- [x] T016 Implement automatic server startup logic (disabled by default, can be enabled)
- [x] T017 Create `kiro-account-manager/src/contexts/LocalServerContext.jsx` for frontend state management
- [x] T018 Create `kiro-account-manager/src/api/server.js` with invoke wrappers for start/stop/status commands

## Phase 4: User Story 2 - Configuration ✅ 100%
**Goal**: Inject configuration (Port, Keys, Path) into the sidecar process.
**Independent Test Criteria**: Sidecar process receives correct env vars and persists data in the correct user directory.

- [x] T019 Implement `get_app_data_dir` logic in `kiro-account-manager/src-tauri/src/server_process.rs` to determine KV storage path
- [x] T020 Update `start_local_server` in `kiro-account-manager/src-tauri/src/server_process.rs` to inject `KIRO_KV_PATH` env var
- [x] T021 Update `start_local_server` in `kiro-account-manager/src-tauri/src/server_process.rs` to inject `PORT` env var (default 7860)
- [x] T022 Update `start_local_server` in `kiro-account-manager/src-tauri/src/server_process.rs` to inject `KIRO_CLIENT_TOKEN` and `KIRO_AUTH_TOKEN` from account store
- [x] T023 Add configuration UI inputs (Port) in `kiro-account-manager/src/components/LocalServer/ConfigPanel.jsx`
- [x] T024 Implement configuration persistence in frontend store (localStorage)

## Phase 5: User Story 3 - Real-time Logs ✅ 100%
**Goal**: Stream stdout/stderr from sidecar to frontend UI.
**Independent Test Criteria**: Frontend displays log lines emitted by the running sidecar process.

- [x] T025 Implement `CommandEvent` listener loop in `kiro-account-manager/src-tauri/src/server_process.rs` to capture sidecar output
- [x] T026 Implement `emit_log_event` helper in `kiro-account-manager/src-tauri/src/server_process.rs` to send logs to frontend
- [x] T027 Create `kiro-account-manager/src/components/LocalServer/LogViewer.jsx` component for displaying logs
- [x] T028 Integrate log event listener in `kiro-account-manager/src/contexts/LocalServerContext.jsx`

## Phase 6: User Story 4 - UI Integration ✅ 100%
**Goal**: Expose functionality to the user via a polished interface.
**Independent Test Criteria**: User can navigate to Local Server page and control the service.

- [x] T029 Create `kiro-account-manager/src/components/LocalServer/LocalServerPage.jsx` aggregating Config and Log components
- [x] T030 Add "Local Server" navigation item to `kiro-account-manager/src/components/Sidebar.jsx`
- [x] T031 Add server status indicator (Green/Red dot) to UI

## Final Phase: Polish ⚠️ Partial
**Goal**: Ensure robust error handling and smooth user experience.
**Independent Test Criteria**: App handles missing binaries or port conflicts gracefully without crashing.

- [x] T032 Handle "Binary not found" error in `start_local_server` and return user-friendly error message
- [ ] T033 Handle "Port already in use" error by detecting immediate process exit and checking stderr
- [x] T034 Verify cross-platform path separators for `KIRO_KV_PATH` on Windows vs macOS

## Implementation Summary

### Completed: 33/34 tasks (97%)

### Files Created/Modified

**Backend (Rust)**
- `src-tauri/src/server_process.rs` - NEW (200+ lines)
- `src-tauri/src/main.rs` - MODIFIED
- `src-tauri/tauri.conf.json` - MODIFIED
- `src-tauri/capabilities/default.json` - MODIFIED

**Frontend (React)**
- `src/contexts/LocalServerContext.jsx` - NEW
- `src/api/server.js` - NEW
- `src/components/LocalServer/LocalServerPage.jsx` - NEW
- `src/components/LocalServer/LogViewer.jsx` - NEW
- `src/components/LocalServer/ConfigPanel.jsx` - NEW
- `src/components/Sidebar.jsx` - MODIFIED
- `src/App.jsx` - MODIFIED

**Server (Deno)**
- `server/` - COPIED from kiro2api-deno
- `server/deno.json` - MODIFIED
- `server/auth/kv_store.ts` - MODIFIED
- `server/main.ts` - MODIFIED

**Scripts & Docs**
- `scripts/build-server.sh` - NEW
- `docs/LOCAL_SERVER.md` - NEW
- `docs/specs/` - COPIED from root specs

### Test Results (2025-12-10)

✅ Server starts successfully
✅ Logs stream to UI in real-time
✅ Status indicator updates correctly
✅ Web dashboard accessible at http://127.0.0.1:7860
✅ Token pool shows 3 tokens (2 active)
✅ Model list returns 6 Claude models
✅ Chat completion API works correctly
✅ Server stops cleanly

### API Test Output

```json
// GET /api/tokens
{
  "total_tokens": 3,
  "active_tokens": 2,
  "tokens": [...]
}

// GET /v1/models
{
  "data": [
    {"id": "claude-sonnet-4-5", ...},
    {"id": "claude-3-7-sonnet-20250219", ...},
    ...
  ]
}

// POST /v1/chat/completions
{
  "choices": [{
    "message": {
      "content": "你好！(Nǐ hǎo!)"
    }
  }]
}
```
