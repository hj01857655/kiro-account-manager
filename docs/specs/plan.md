# Implementation Plan - Local Kiro 2API Sidecar Integration

**Status**: Draft
**Feature**: [Link to Spec](./spec.md)

## Technical Context

This feature involves integrating the existing `kiro2api-deno` service as a local sidecar within the `kiro-account-manager` Tauri application.

**Knowns:**
- **Source Code**: `kiro2api-deno` code will be migrated to `kiro-account-manager/server`.
- **Runtime**: Deno will be used to compile the server code into a standalone binary.
- **Process Management**: Tauri's `externalBin` (Sidecar) pattern will be used to manage the binary.
- **IPC**: Tauri `Command` and `Event` system will handle communication (start/stop, logs).
- **Storage**: Deno KV will be used, with the path configurable via environment variable `KIRO_KV_PATH` to point to the user's AppData directory.
- **Network**: The service will listen on `localhost` with a configurable port (default 7860).

**Unknowns & Risks:**
- **Cross-Platform Compilation**: While initial focus is macOS, ensuring `deno compile` works seamlessly across targets (Windows/Linux) in CI might require specific setup.
- **Binary Size**: The compiled binary might significantly increase the installer size.
- **Port Conflicts**: Although 7860 is chosen, handling conflicts gracefully (e.g., auto-finding an open port or alerting the user) needs to be considered.

## Constitution Check

- [x] **Principle 1**: [N/A] (No specific project principles defined in placeholder constitution)
- [x] **Principle 2**: [N/A]
- [x] **Principle 3**: [N/A]
- [x] **Principle 4**: [N/A]
- [x] **Principle 5**: [N/A]

**Result**: PASS (Placeholder constitution does not impose specific constraints yet).

## Phase 0: Research & Decisions

### 1. Research Tasks

- **Task 0.1**: Verify `deno compile` capabilities for including static assets (if any) or if they need to be handled differently (e.g., `kiro2api-deno` serves some static files).
    - *Decision*: Check if `deno compile` supports `--include` or if we need to bundle assets into the binary or serve them from a separate directory.
    - *Status*: Pending Research.

- **Task 0.2**: Confirm Tauri Sidecar path resolution in production builds vs. development environment.
    - *Decision*: Ensure correct path mapping for `externalBin` in `tauri.conf.json`.
    - *Status*: Standard Tauri pattern, but needs verification for the specific directory structure.

### 2. Technology Decisions

| Area | Decision | Rationale | Alternatives |
|---|---|---|---|
| **Runtime** | Deno Compile | Allows running TS code as a native binary without requiring user to install Deno. | Node.js (pkg), Rust rewrite (too expensive). |
| **Process Mgmt** | Tauri Sidecar | Native Tauri feature for managing external binaries, handles lifecycle and permissions. | Spawning raw processes (less integrated). |
| **Communication** | Stdout/Stderr | Simple, text-based log streaming from sidecar to parent. | Named pipes / Sockets (overkill for simple logging). |
| **Config** | Env Vars | Standard way to pass config (Port, Keys, Path) to the process. | CLI arguments (also viable, Env is cleaner for secrets). |

## Phase 1: Design & Contracts

### 1. Data Model (`data-model.md`)

*   **Entities**:
    *   `LocalServerConfig`: Port, AutoStart (boolean), DataPath.
    *   `ServerStatus`: 'Stopped', 'Starting', 'Running', 'Error'.
    *   `LogEntry`: Timestamp, Level, Message, Source (stdout/stderr).

### 2. API Contracts (`contracts/`)

*   **Internal (Rust <-> Frontend)**:
    *   `start_local_server()`: Invoke -> Result<void, string>
    *   `stop_local_server()`: Invoke -> Result<void, string>
    *   `get_server_status()`: Invoke -> Result<ServerStatus, string>
    *   `save_server_config(config)`: Invoke -> Result<void, string>
    *   `event:server-log`: Event payload `{ message: string, type: 'stdout'|'stderr' }`
    *   `event:server-status-change`: Event payload `{ status: ServerStatus }`

### 3. Quickstart (`quickstart.md`)

*   Instructions for developers to:
    1.  Install Deno (for compilation).
    2.  Run the compile script to generate the binary.
    3.  Run the Tauri app in dev mode with the sidecar attached.

## Phase 2: Implementation Breakdown

### 1. Backend (Deno) Migration & Adaptation
- Move code to `kiro-account-manager/server`.
- Modify `auth/kv_store.ts` to support `KIRO_KV_PATH`.
- Add `deno compile` script to `deno.json`.

### 2. Tauri Host (Rust)
- Configure `tauri.conf.json` for `externalBin`.
- Implement `server_process.rs` module for process management.
- Expose Tauri commands (`start`, `stop`, `config`).
- Implement log streaming logic.

### 3. Frontend (React)
- Create `LocalServer` context/store.
- Build `LocalServerPanel` component (Controls + Log View).
- Integrate into Sidebar and Main Layout.

### 4. Integration & Testing
- Verify start/stop flows.
- Verify log streaming.
- Verify API conversion via `localhost:7860`.
- Verify data persistence in AppData.