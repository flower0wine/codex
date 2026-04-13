# Packaging Guide

This document records the packaging method used for the standalone Windows
sandbox artifacts.

## Goal

Produce runnable sandbox binaries without requiring `codex.exe`:

- `codex-windows-sandbox-host.exe`
- `codex-windows-sandbox-setup.exe`
- `codex-command-runner.exe`

## Build steps

From repo root:

```powershell
cd C:\github\codex\codex-rs
cargo build -p codex-windows-sandbox --bins
```

Release build (recommended for distribution):

```powershell
cd C:\github\codex\codex-rs
cargo build -p codex-windows-sandbox --bins --release
```

## Validation steps used

Unit/integration tests for the crate:

```powershell
cd C:\github\codex\codex-rs
cargo test -p codex-windows-sandbox
```

Smoke checks for standalone host:

```powershell
.\target\debug\codex-windows-sandbox-host.exe --help
.\target\debug\codex-windows-sandbox-host.exe --policy workspace-write -- cmd /c "echo HOST_OK"
```

## Minimal distribution layout

Create a folder (example: `C:\dist\windows-sandbox`) and copy:

- `target\release\codex-windows-sandbox-host.exe`
- `target\release\codex-windows-sandbox-setup.exe`
- `target\release\codex-command-runner.exe`

All three files should stay in the same directory.

## First-run notes

- Runtime state is created under `CODEX_HOME` (or default fallback).
- Typical runtime folders:
  - `.sandbox`
  - `.sandbox-bin`
  - `.sandbox-secrets`
- If running on a clean machine, ensure required MSVC runtime is available.

## Example packaged usage

```powershell
setx CODEX_HOME C:\sandbox-home
C:\dist\windows-sandbox\codex-windows-sandbox-host.exe --policy workspace-write -- cmd /c "echo OK"
```

