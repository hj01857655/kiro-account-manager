# Feature Specification: Local Kiro 2API Sidecar Integration

## 1. Feature Name
Local Kiro 2API Sidecar Integration

## 2. Feature Description
This feature integrates the `kiro2api-deno` service directly into the `kiro-account-manager` desktop application. It enables users to run the OpenAI-to-Anthropic API conversion service locally as a background process (sidecar) managed by the desktop client. This eliminates the need for an external, separately deployed `kiro2api-deno` instance, enhancing user privacy, responsiveness, and simplifying setup.

## 3. User Stories
*   As a Kiro IDE user, I want to start and stop the local 2API conversion service from the Kiro Account Manager UI, so that I can use Anthropic models with my OpenAI-compatible tools without external dependencies.
*   As a Kiro IDE user, I want to configure the port and data storage path for the local 2API service, so that it integrates seamlessly with my local development environment and doesn't conflict with other services.
*   As a Kiro IDE user, I want to view real-time logs from the local 2API service within the Kiro Account Manager UI, so that I can monitor its operation and diagnose issues.
*   As a Kiro IDE user, I want the local 2API service to automatically use my configured Anthropic API keys (managed by Kiro Account Manager), so that I don't have to manually re-enter them.
*   As a Kiro IDE user, I want the local 2API service to run without requiring a separate Deno installation on my system, so that setup is simplified.

## 4. Functional Requirements

### FR1: Local 2API Service Management
*   The system SHALL provide a mechanism to start the local Kiro 2API service.
*   The system SHALL provide a mechanism to stop the local Kiro 2API service.
*   The system SHALL display the current running status (e.g., running, stopped, error) of the local Kiro 2API service in the UI.

### FR2: Service Configuration
*   The system SHALL allow users to configure the local HTTP port on which the 2API service listens.
*   The system SHALL use a dedicated, user-specific data directory (e.g., within AppData/Application Support) for the local 2API service's persistent storage (e.g., Deno KV store).
*   The system SHALL inject necessary environment variables (e.g., `KIRO_CLIENT_TOKEN`, `Anthropic API keys`) into the local 2API service process.

### FR3: Real-time Logging
*   The system SHALL capture and display `stdout` and `stderr` output from the local 2API service in a dedicated UI section.
*   The system SHALL provide controls (e.g., clear, scroll to bottom) for the log display.

### FR4: Dependency Management
*   The local 2API service binary SHALL be bundled with the Kiro Account Manager application.
*   The local 2API service SHALL run without requiring a pre-installed Deno runtime on the user's system.

## 5. Non-Functional Requirements (Optional)
*   **Performance**: The local 2API service SHOULD respond to OpenAI API calls with minimal additional latency compared to direct Anthropic API calls, accounting for local proxying overhead.
*   **Resource Usage**: The local 2API service SHOULD have a minimal CPU and memory footprint when idle.
*   **Security**: The local 2API service SHOULD only be accessible from `localhost` by default.

## 6. Success Criteria
*   Users can successfully start the local Kiro 2API service from the Kiro Account Manager UI, and it becomes accessible via `localhost:<configured_port>`.
*   Users can successfully stop the local Kiro 2API service from the Kiro Account Manager UI.
*   All API requests made to the local 2API service (`localhost:<configured_port>/v1/chat/completions`) are successfully converted to Anthropic API calls and responses are returned correctly, as verified by an OpenAI-compatible client.
*   The local 2API service persistently stores its data (e.g., token usage) within the application's dedicated user data directory.
*   The Kiro Account Manager UI accurately reflects the running status of the local 2API service.
*   Users can view streamed logs from the local 2API service in the Kiro Account Manager UI, providing visibility into its operation.
*   The installation and first run of Kiro Account Manager with the integrated 2API sidecar does not require manual Deno installation by the user.

## 7. Assumptions
*   The `kiro2api-deno` codebase is suitable for compilation into a single, standalone binary for each target operating system (macOS, Windows, Linux).
*   The `kiro-account-manager` application has the necessary permissions to execute child processes and manage network ports.
*   The Anthropic API keys will be securely managed and injected by the `kiro-account-manager` into the sidecar process.
*   Initial development and testing will prioritize macOS. Windows and Linux support for the sidecar binary will be considered in future iterations or require cross-compilation setup.

## 8. Open Questions / Clarifications Needed
*   默认端口为 `7860`。
*   本地 2API 服务将随 Kiro Account Manager 应用程序自动启动。

## 9. User Interface (Optional)
*   A new navigation item in the Kiro Account Manager sidebar, e.g., "Local Kiro 2API Service".
*   A dedicated panel for the service with:
    *   Start/Stop button.
    *   Status indicator (e.g., green/red circle).
    *   Port configuration input field.
    *   A read-only log output area (scrolling, clear button).

## 10. Technical Notes (Optional)
*   The Rust backend (Tauri) will use `Command::new_sidecar` to manage the Deno binary.
*   Environment variables will be passed to the Deno child process for configuration.
*   `Deno.run` (or similar for child processes) will be used to capture `stdout` and `stderr`.
*   Frontend will use Tauri's `invoke` for commands and `listen` for events.