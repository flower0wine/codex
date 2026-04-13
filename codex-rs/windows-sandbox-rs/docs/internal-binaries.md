# Internal Binaries

This file documents the two helper executables used by the elevated sandbox
path. They are not intended as public CLIs.

## `codex-windows-sandbox-setup.exe`

### Purpose

Performs sandbox setup/refresh tasks (users, ACLs, firewall rules, marker
files, protected directories).

### Invocation contract

- Expects exactly one argument.
- The argument must be base64-encoded JSON payload.
- Any other invocation fails with `helper_request_args_failed`.

Payload shape (derived from `setup_main_win.rs`):

```json
{
  "version": 5,
  "offline_username": "CodexSandboxOffline",
  "online_username": "CodexSandboxOnline",
  "codex_home": "C:\\path\\to\\codex-home",
  "command_cwd": "C:\\path\\to\\cwd",
  "read_roots": ["..."],
  "write_roots": ["..."],
  "deny_write_paths": ["..."],
  "proxy_ports": [3128],
  "allow_local_binding": false,
  "real_user": "YourUser",
  "mode": "full",
  "refresh_only": false
}
```

`mode` values:

- `full`
- `read-acls-only`

### Files written under `CODEX_HOME`

- `.sandbox\sandbox.log`
- `.sandbox\setup_error.json` (on structured setup failure)
- `.sandbox\setup_marker.json`
- `.sandbox-secrets\sandbox_users.json`
- `.sandbox-bin\...` helper copy targets

## `codex-command-runner.exe`

### Purpose

Executes child commands under sandbox credentials in elevated path, speaking
framed JSON over named pipes.

### Invocation contract

Required args:

- `--pipe-in=<PIPE_NAME>`
- `--pipe-out=<PIPE_NAME>`

If either is missing, process exits with:

- `runner: no pipe-in provided`
- `runner: no pipe-out provided`

### IPC protocol

Defined in `src/elevated/ipc_framed.rs`:

- Frame format: little-endian u32 length + JSON payload.
- Protocol version: `1`.
- Parent -> runner:
  - `spawn_request`
  - `stdin`
  - `terminate`
- Runner -> parent:
  - `spawn_ready`
  - `output`
  - `exit`
  - `error`

Important `spawn_request` fields:

- `command`, `cwd`, `env`
- `policy_json_or_preset`
- `sandbox_policy_cwd`
- `codex_home`, `real_codex_home`
- `cap_sids`
- `timeout_ms`
- `tty`, `stdin_open`, `use_private_desktop`

### Should you call it directly?

Normally no. Use one of:

- `codex-windows-sandbox-host.exe` (standalone CLI)
- Rust API (`run_windows_sandbox_capture_elevated`)
