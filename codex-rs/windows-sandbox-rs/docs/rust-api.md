# Rust API Integration

This crate can be embedded directly from Rust without using `codex.exe`.

## Main APIs

### Elevated path (recommended for Windows)

```rust
use codex_windows_sandbox::ElevatedSandboxCaptureRequest;
use codex_windows_sandbox::run_windows_sandbox_capture_elevated;
```

Entry:

- `run_windows_sandbox_capture_elevated(request)`  
  (re-export of `elevated_impl::run_windows_sandbox_capture`)

### Legacy path (non-elevated execution path)

```rust
use codex_windows_sandbox::run_windows_sandbox_capture;
use codex_windows_sandbox::run_windows_sandbox_capture_with_extra_deny_write_paths;
use codex_windows_sandbox::run_windows_sandbox_legacy_preflight;
```

## `ElevatedSandboxCaptureRequest`

Required fields:

- `policy_json_or_preset: &str`
- `sandbox_policy_cwd: &Path`
- `codex_home: &Path`
- `command: Vec<String>`
- `cwd: &Path`
- `env_map: HashMap<String, String>`

Optional/behavioral fields:

- `timeout_ms: Option<u64>`
- `use_private_desktop: bool`
- `proxy_enforced: bool`
- `read_roots_override: Option<&[PathBuf]>`
- `write_roots_override: Option<&[PathBuf]>`
- `deny_write_paths_override: &[PathBuf]`

## Return type

Capture result contains:

- `exit_code: i32`
- `stdout: Vec<u8>`
- `stderr: Vec<u8>`
- `timed_out: bool`

## Minimal example

```rust
use std::collections::HashMap;
use std::path::Path;

use codex_windows_sandbox::ElevatedSandboxCaptureRequest;
use codex_windows_sandbox::run_windows_sandbox_capture_elevated;

fn main() -> anyhow::Result<()> {
    let request = ElevatedSandboxCaptureRequest {
        policy_json_or_preset: "workspace-write",
        sandbox_policy_cwd: Path::new("C:\\work"),
        codex_home: Path::new("C:\\work\\.codex"),
        command: vec!["cmd".into(), "/c".into(), "echo API_OK".into()],
        cwd: Path::new("C:\\work"),
        env_map: HashMap::new(),
        timeout_ms: None,
        use_private_desktop: false,
        proxy_enforced: false,
        read_roots_override: None,
        write_roots_override: None,
        deny_write_paths_override: &[],
    };

    let capture = run_windows_sandbox_capture_elevated(request)?;
    print!("{}", String::from_utf8_lossy(&capture.stdout));
    eprint!("{}", String::from_utf8_lossy(&capture.stderr));
    std::process::exit(capture.exit_code);
}
```

## Policy values

`policy_json_or_preset` accepts:

- preset string: `read-only`, `workspace-write`
- JSON string for `SandboxPolicy` (from `codex-protocol`)

Rejected in this sandbox execution path:

- `danger-full-access`
- `external-sandbox`

## Runtime files and diagnostics

Under `codex_home`:

- `.sandbox\sandbox.log`
- `.sandbox\setup_error.json` (when setup fails)
- `.sandbox\setup_marker.json`
- `.sandbox-bin\...`
- `.sandbox-secrets\...`
