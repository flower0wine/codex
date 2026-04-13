# `codex-windows-sandbox-host.exe`

`codex-windows-sandbox-host.exe` is the supported standalone CLI for executing
commands through the Windows sandbox crate.

It wraps `run_windows_sandbox_capture_elevated(...)` and prints child
`stdout/stderr` directly.

## Usage

```text
codex-windows-sandbox-host [OPTIONS] -- <COMMAND> [ARGS...]
```

Example:

```powershell
.\codex-windows-sandbox-host.exe --policy workspace-write -- cmd /c "echo HOST_OK"
```

## Options

- `--policy <VALUE>`
  - Default: `read-only`
  - Accepted values:
    - `read-only`
    - `workspace-write`
    - JSON object matching `SandboxPolicy` (see below)
- `--policy-cwd <PATH>`
  - Base directory for policy-relative interpretation.
  - Default: `--cwd` value.
- `--cwd <PATH>`
  - Working directory for the spawned command.
  - Default: current process directory.
- `--codex-home <PATH>`
  - Sandbox runtime state location.
  - Default resolution order:
    1. explicit `--codex-home`
    2. `CODEX_HOME` env var
    3. `<home>\.codex`
    4. `<cwd>\.codex`
- `--timeout-ms <U64>`
  - Optional timeout in milliseconds.
- `--private-desktop`
  - Requests private desktop mode for elevated path.
- `--proxy-enforced`
  - Forces offline sandbox identity path used by setup/network policy.
- `--read-root <PATH>` (repeatable)
  - Optional override list for readable roots.
- `--write-root <PATH>` (repeatable)
  - Optional override list for writable roots.
- `--deny-write-path <PATH>` (repeatable)
  - Optional explicit deny-write subpaths.
- `--env <KEY=VALUE>` (repeatable)
  - Add/override child environment variable.
- `--clear-env`
  - Start from empty env map before applying `--env`.

## Exit code and output

- Child process `stdout/stderr` are streamed to the host process outputs.
- Host exits with child exit code.
- If timeout occurs, the child is terminated and host exits with timeout code
  behavior from capture path.

## `SandboxPolicy` JSON examples

Preset strings:

- `read-only`
- `workspace-write`

JSON examples:

Read-only with restricted read roots and no platform defaults:

```json
{
  "type": "read-only",
  "access": {
    "type": "restricted",
    "include_platform_defaults": false,
    "readable_roots": ["C:\\work\\project", "C:\\tools\\readonly"]
  },
  "network_access": false
}
```

Workspace-write with extra writable root:

```json
{
  "type": "workspace-write",
  "writable_roots": ["C:\\work\\shared"],
  "read_only_access": {
    "type": "full-access"
  },
  "network_access": false,
  "exclude_tmpdir_env_var": false,
  "exclude_slash_tmp": false
}
```

Notes:

- `danger-full-access` and `external-sandbox` are rejected by this crate's
  sandbox execution path.
- Policy parsing follows `codex_protocol::protocol::SandboxPolicy`.

## Common troubleshooting

- `unknown argument ...`: make sure command args are after `--`.
- `missing command. Use -- <COMMAND> [ARGS...]`: you forgot command separator.
- Setup-related failures are written under `CODEX_HOME\.sandbox`:
  - `sandbox.log`
  - `setup_error.json` (when setup fails)
