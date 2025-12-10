# Research Findings: Local Kiro 2API Sidecar Integration

**Feature**: [Link to Spec](./spec.md)
**Status**: Completed

## 1. Deno Compile & Static Assets

**Question**: Does `deno compile` support including static assets (HTML/CSS/JS) into the single binary, or do we need a workaround?

**Findings**:
- `deno compile` supports the `--include <path>` flag.
- This allows embedding files or directories directly into the executable.
- The embedded files are accessible via standard Deno file APIs (e.g., `Deno.readFile`, `Deno.readTextFile`) at runtime.
- **Example**: `deno compile --allow-net --allow-read --include server/static server/main.ts`

**Decision**:
- We will use the `--include` flag to bundle the `static/` directory (Web Dashboard) into the `kiro-server` binary.
- This ensures the sidecar remains a single, self-contained file without external file dependencies.
- No code changes are required in `serveStaticFile` logic as Deno handles the virtual file system transparently for embedded files.

## 2. Tauri Sidecar Path Resolution

**Question**: How to ensure the sidecar binary is correctly located in both development and production?

**Findings**:
- Tauri uses a naming convention for sidecars: `name-target-triple`.
- Example for macOS (Intel): `kiro-server-x86_64-apple-darwin`.
- In `tauri.conf.json`:
  ```json
  "bundle": {
    "externalBin": ["bin/kiro-server"]
  }
  ```
- Tauri expects the binaries to exist at `src-tauri/bin/` during build time.
- At runtime (prod), Tauri looks in the `resources` directory alongside the app executable.
- At runtime (dev), it looks in the same `bin` folder.

**Decision**:
- We will create a `src-tauri/bin` directory.
- We will write a helper script (`scripts/build-server.sh`) to:
    1.  Run `deno compile` with target triples.
    2.  Rename the output binaries to match Tauri's expectation (`kiro-server-$TARGET_TRIPLE`).
    3.  Move them to `src-tauri/bin/`.
- This ensures seamless integration for both dev (local compilation) and prod (CI/CD cross-compilation).

## 3. Port Management

**Question**: How to handle default port (7860) conflicts?

**Decision**:
- **Initial Implementation**: Use strict port 7860 as defined in Spec.
- If the port is in use, the Deno process will likely fail to start and exit.
- **Error Handling**: The Rust sidecar manager will capture the process exit and stderr.
- We will surface this error to the user in the "Log" panel: "Port 7860 is already in use."
- **Future Improvement**: Add logic to `server/main.ts` to try the configured port, and if failed, exit with a specific code, or allow passing `0` for a random port (though random port makes client config harder). For now, strict port + error message is sufficient for MVP.
