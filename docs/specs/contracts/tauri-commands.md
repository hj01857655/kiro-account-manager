# Tauri IPC Contracts

## Commands (Frontend -> Backend)

### `start_local_server`
Starts the Deno sidecar process with the current configuration.

*   **Input**: None (Uses saved config or defaults)
*   **Output**: `Result<(), string>`
    *   `Ok(())`: Process started successfully.
    *   `Err(msg)`: Failed to start (e.g., port in use, binary not found).

### `stop_local_server`
Stops the running Deno sidecar process.

*   **Input**: None
*   **Output**: `Result<(), string>`

### `get_server_status`
Gets the current status of the server process.

*   **Input**: None
*   **Output**: `Result<ServerStatus, string>`
    *   `ServerStatus`: "Stopped" | "Starting" | "Running" | "Error"

### `save_server_config`
Updates the server configuration. Requires restart to take effect if server is running.

*   **Input**: `config: LocalServerConfig`
    *   `port`: u16
    *   `auto_start`: bool
*   **Output**: `Result<(), string>`

### `get_server_config`
Retrieves the current server configuration.

*   **Input**: None
*   **Output**: `Result<LocalServerConfig, string>`

## Events (Backend -> Frontend)

### `server-log`
Emitted whenever the sidecar process writes to stdout or stderr.

*   **Payload**:
    ```json
    {
      "source": "stdout" | "stderr",
      "line": "string"
    }
    ```

### `server-status-change`
Emitted when the server status changes (e.g., process exited unexpectedly).

*   **Payload**:
    ```json
    {
      "status": "Stopped" | "Starting" | "Running" | "Error",
      "message": "Optional status message (e.g. exit code)"
    }
    ```
