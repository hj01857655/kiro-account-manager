# Quickstart: Local Kiro 2API Sidecar Integration

## Prerequisites

1.  **Deno**: You must have Deno installed to compile the server binary.
    ```bash
    curl -fsSL https://deno.land/x/install/install.sh | sh
    ```
2.  **Rust/Tauri**: Standard environment for building the desktop app.

## Build Instructions

### 1. Compile the Sidecar
Run the helper script to compile the `kiro2api-deno` code into a binary compatible with your current OS.

```bash
# From project root
cd kiro-account-manager
./scripts/build-server.sh
```
*This will create `src-tauri/bin/kiro-server-<target-triple>`.*

### 2. Run the Application
Start the Tauri application in development mode.

```bash
npm run tauri dev
```

## Usage

1.  Open the Kiro Account Manager app.
2.  Navigate to the "Local Server" tab in the sidebar.
3.  Click "Start Server".
4.  Observe the logs in the terminal window within the UI.
5.  Test the API:
    ```bash
    curl http://localhost:7860/v1/models
    ```

## Troubleshooting

*   **"Binary not found"**: Ensure you ran `./scripts/build-server.sh` and that the filename in `src-tauri/bin/` matches your platform's target triple (e.g., `x86_64-apple-darwin`).
*   **"Port already in use"**: Check if another service is using port 7860, or change the port in the UI settings.
*   **Logs not showing**: Check the Tauri console output for any IPC errors.
