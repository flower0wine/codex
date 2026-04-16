# Windows Sandbox Docs

This directory documents how to use the Windows sandbox artifacts in
`codex-rs/windows-sandbox-rs` without requiring the full Codex app.

## What you can run directly

- `codex-windows-sandbox-host.exe` (recommended public entrypoint)
- `codex-windows-sandbox-setup.exe` (internal helper, payload-driven)
- `codex-command-runner.exe` (internal helper, named-pipe-driven)

## Usage methods matrix

- Full Codex CLI:
  - `codex.exe sandbox windows ...`
  - Best when you already use Codex end-to-end.
- Standalone host CLI:
  - `codex-windows-sandbox-host.exe ...`
  - Best when you want sandbox capability without full Codex UX.
- Rust embedding:
  - Call `run_windows_sandbox_capture_elevated(...)` directly.
  - Best for integrating sandbox execution into your own app/service.
- Internal helpers (not public APIs):
  - `codex-windows-sandbox-setup.exe`
  - `codex-command-runner.exe`

## Quick start

Build binaries:

```powershell
cd C:\github\codex\codex-rs
cargo build -p codex-windows-sandbox --bins
```

Run a command with sandboxing:

```powershell
cd C:\github\codex\codex-rs
.\target\debug\codex-windows-sandbox-host.exe --policy workspace-write -- cmd /c "echo hello"
```

## Required files for a minimal distribution

Put these three binaries in the same directory:

- `codex-windows-sandbox-host.exe`
- `codex-windows-sandbox-setup.exe`
- `codex-command-runner.exe`

Runtime state will be created under `CODEX_HOME` (or its default):

- `.sandbox`
- `.sandbox-bin`
- `.sandbox-secrets`

## Documents in this folder

- `sandbox-host.md`: command-line reference for `codex-windows-sandbox-host.exe`
- `internal-binaries.md`: behavior and wire contracts for setup/runner helpers
- `rust-api.md`: Rust API integration (`run_windows_sandbox_capture_elevated`)
- `packaging.md`: reproducible build/package/validation steps for distribution
- `zh-CN/README.md`: 中文文档导航（`sandbox-host` 使用、参数与策略说明、排障）
