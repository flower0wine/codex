# `codex-windows-sandbox-host.exe`

`codex-windows-sandbox-host.exe` is the supported standalone CLI for executing
commands through the Windows sandbox crate.

It wraps `run_windows_sandbox_capture_elevated(...)` and prints child
`stdout/stderr` directly.

## Usage

```text
codex-windows-sandbox-host [OPTIONS] -- <COMMAND> [ARGS...]
```

Examples:

```powershell
.\codex-windows-sandbox-host.exe --backend auto --policy workspace-write -- cmd /c "echo HOST_OK"
.\codex-windows-sandbox-host.exe --backend unelevated --policy workspace-write -- cmd /c "echo HOST_OK"
.\codex-windows-sandbox-host.exe --backend elevated --policy workspace-write -- cmd /c "echo HOST_OK"
.\codex-windows-sandbox-host.exe --policy workspace-write -- cmd /c "echo HOST_OK"
.\codex-windows-sandbox-host.exe --clear-env --env PATH=C:\Windows\System32 -- cmd /c ver
.\codex-windows-sandbox-host.exe --policy-cwd C:\work --policy "{\"type\":\"read-only\"}" -- powershell -NoProfile -Command Get-ChildItem
```

## Options

- `-h`, `--help`
  - Print full help text.
- `--policy <VALUE>`
  - Default: `read-only`
  - Accepted values:
    - `read-only`
    - `workspace-write`
    - JSON object matching `SandboxPolicy` (see below)
  - Rejected values:
    - `danger-full-access`
    - `external-sandbox`
- `--backend <auto|elevated|unelevated>`
  - Default: `auto`
  - `elevated`: force elevated setup/runner path.
  - `unelevated`: force restricted-token path (no UAC setup flow).
  - `auto` selection:
    1. Use `elevated` when `--proxy-enforced` is set.
    2. Use `elevated` when `--read-root` or `--write-root` is provided.
    3. Use `elevated` when policy requires restricted read access.
    4. Otherwise use `elevated` only if sandbox setup marker already exists.
    5. Fall back to `unelevated` when setup is not complete.
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
  - If timeout happens, host prints `sandbox command timed out` and exits with `192`.
- `--private-desktop`
  - Requests private desktop mode for elevated path.
- `--proxy-enforced`
  - Forces offline sandbox identity path used by setup/network policy.
  - Supported only by `elevated` backend.
- `--read-root <PATH>` (repeatable)
  - Optional override list for readable roots.
  - If at least one `--read-root` is provided, computed defaults are replaced by this list.
  - Supported only by `elevated` backend.
- `--write-root <PATH>` (repeatable)
  - Optional override list for writable roots.
  - If at least one `--write-root` is provided, computed defaults are replaced by this list.
  - Supported only by `elevated` backend.
- `--deny-write-path <PATH>` (repeatable)
  - Optional explicit deny-write subpaths.
- `--deny-read-path <PATH>` (repeatable)
  - Optional explicit deny-read subpaths.
  - Deny-read wins over `--read-root`.
  - Supported only by `elevated` backend.
- `--temp-root <PATH>`
  - Creates/uses an isolated child temp directory.
  - Sets child `TEMP` and `TMP` to this path.
  - Adds this path to the writable roots.
  - Host `%TEMP%` and `%TMP%` are not made writable unless they are the same path.
- `--network <none|default>`
  - `default`: follow the sandbox policy.
  - `none`: force the offline sandbox identity and outbound firewall block.
  - Supported only by `elevated` backend.
- `--capabilities --json`
  - Prints passive capability JSON and exits.
- `--probe --json`
  - Prints capability JSON and exits. Active probes are not run by this command yet.
- `--env <KEY=VALUE>` (repeatable)
  - Add/override child environment variable.
- `--clear-env`
  - Start from empty env map before applying `--env`.
  - Option ordering matters. If `--clear-env` appears after `--env`, earlier `--env` values are cleared too.

## Argument parsing rules

- All sandbox-host options must appear before `--`.
- Everything after `--` is treated as command + command args.
- Unknown options fail fast.
- Omitting command causes:
  - `missing command. Use -- <COMMAND> [ARGS...]`

## Exit code and output

- Child process `stdout/stderr` are streamed to the host process outputs.
- Host exits with child exit code.
- If timeout occurs, the child is terminated and host exits with `192`.

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
