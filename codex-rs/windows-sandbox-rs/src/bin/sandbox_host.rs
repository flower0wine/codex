#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use codex_windows_sandbox::ElevatedSandboxCaptureRequest;
    use codex_windows_sandbox::run_windows_sandbox_capture_elevated;
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::PathBuf;

    let mut args = std::env::args().skip(1);
    let mut policy = String::from("read-only");
    let mut policy_cwd: Option<PathBuf> = None;
    let mut cwd: Option<PathBuf> = None;
    let mut codex_home: Option<PathBuf> = None;
    let mut timeout_ms: Option<u64> = None;
    let mut use_private_desktop = false;
    let mut proxy_enforced = false;
    let mut read_roots: Vec<PathBuf> = Vec::new();
    let mut write_roots: Vec<PathBuf> = Vec::new();
    let mut deny_write_paths: Vec<PathBuf> = Vec::new();
    let mut env_map: HashMap<String, String> = std::env::vars().collect();
    let mut command: Vec<String> = Vec::new();

    const HELP_TEXT: &str = "\
codex-windows-sandbox-host - run a command through Windows sandbox setup + runner

Usage:
  codex-windows-sandbox-host [OPTIONS] -- <COMMAND> [ARGS...]

Examples:
  codex-windows-sandbox-host --policy workspace-write -- cmd /c \"echo HOST_OK\"
  codex-windows-sandbox-host --clear-env --env PATH=C:\\Windows\\System32 -- cmd /c ver
  codex-windows-sandbox-host --policy-cwd C:\\work --policy \"{\\\"type\\\":\\\"read-only\\\"}\" -- powershell -NoProfile -Command Get-ChildItem

Options:
  -h, --help
      Show this help text.

  --policy <read-only|workspace-write|JSON>
      Sandbox policy preset or raw SandboxPolicy JSON.
      Default: read-only.
      Notes:
      - danger-full-access and external-sandbox are rejected.
      - JSON is parsed exactly as codex_protocol::protocol::SandboxPolicy.

  --policy-cwd <PATH>
      Base directory used when resolving policy-relative paths.
      Default: value of --cwd (or current process directory if --cwd is not set).

  --cwd <PATH>
      Working directory of the child command inside the sandbox session.
      Default: current process directory.

  --codex-home <PATH>
      Base directory that stores sandbox runtime artifacts.
      Default resolution order:
      1) --codex-home
      2) CODEX_HOME
      3) %USERPROFILE%\\.codex
      4) <cwd>\\.codex

  --timeout-ms <U64>
      Optional timeout in milliseconds. If exceeded, child is terminated.
      Host prints \"sandbox command timed out\" and exits with code 192.

  --private-desktop
      Request private desktop mode for TTY/conpty launches.

  --proxy-enforced
      Force offline network identity path during setup/refresh, even if policy
      requests full network access.

  --read-root <PATH>      (repeatable)
      Override computed readable roots with an explicit root list.
      When present at least once, only the provided values are used.

  --write-root <PATH>     (repeatable)
      Override computed writable roots with an explicit root list.
      When present at least once, only the provided values are used.

  --deny-write-path <PATH> (repeatable)
      Add explicit deny-write paths that remain read-only even under
      workspace-write configurations.

  --env <KEY=VALUE>       (repeatable)
      Set or override one environment variable for the child process.
      By default, the current process environment is inherited first.

  --clear-env
      Clear inherited environment before applying any --env pairs.
      Option ordering matters: later --clear-env clears earlier --env values.

Argument parsing notes:
  - All sandbox-host options must appear before \"--\".
  - Everything after \"--\" is treated as command + command args.
  - Unknown options fail fast.
";

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => {
                command.extend(args);
                break;
            }
            "-h" | "--help" => {
                println!("{HELP_TEXT}");
                return Ok(());
            }
            "--policy" => {
                policy = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for --policy"))?;
            }
            "--policy-cwd" => {
                policy_cwd =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --policy-cwd")
                    })?));
            }
            "--cwd" => {
                cwd = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --cwd"))?,
                ));
            }
            "--codex-home" => {
                codex_home =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --codex-home")
                    })?));
            }
            "--timeout-ms" => {
                timeout_ms = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --timeout-ms"))?
                        .parse::<u64>()
                        .map_err(|err| anyhow::anyhow!("invalid --timeout-ms: {err}"))?,
                );
            }
            "--private-desktop" => {
                use_private_desktop = true;
            }
            "--proxy-enforced" => {
                proxy_enforced = true;
            }
            "--read-root" => {
                read_roots
                    .push(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --read-root")
                    })?));
            }
            "--write-root" => {
                write_roots
                    .push(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --write-root")
                    })?));
            }
            "--deny-write-path" => {
                deny_write_paths
                    .push(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --deny-write-path")
                    })?));
            }
            "--env" => {
                let kv = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for --env"))?;
                let (key, value) = kv
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("--env expects KEY=VALUE, got {kv}"))?;
                env_map.insert(key.to_string(), value.to_string());
            }
            "--clear-env" => {
                env_map.clear();
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "unknown argument: {arg}. Use --help for usage."
                ));
            }
        }
    }

    if command.is_empty() {
        return Err(anyhow::anyhow!(
            "missing command. Use -- <COMMAND> [ARGS...]"
        ));
    }

    let cwd = cwd.unwrap_or(std::env::current_dir()?);
    let policy_cwd = policy_cwd.unwrap_or_else(|| cwd.clone());
    let codex_home = codex_home
        .or_else(|| std::env::var_os("CODEX_HOME").map(PathBuf::from))
        .or_else(|| dirs_next::home_dir().map(|home| home.join(".codex")))
        .unwrap_or_else(|| cwd.join(".codex"));
    let read_roots_override = if read_roots.is_empty() {
        None
    } else {
        Some(read_roots.as_slice())
    };
    let write_roots_override = if write_roots.is_empty() {
        None
    } else {
        Some(write_roots.as_slice())
    };

    let capture = run_windows_sandbox_capture_elevated(ElevatedSandboxCaptureRequest {
        policy_json_or_preset: policy.as_str(),
        sandbox_policy_cwd: policy_cwd.as_path(),
        codex_home: codex_home.as_path(),
        command,
        cwd: cwd.as_path(),
        env_map,
        timeout_ms,
        use_private_desktop,
        proxy_enforced,
        read_roots_override,
        write_roots_override,
        deny_write_paths_override: deny_write_paths.as_slice(),
    })?;

    if !capture.stdout.is_empty() {
        std::io::stdout().write_all(&capture.stdout)?;
    }
    if !capture.stderr.is_empty() {
        std::io::stderr().write_all(&capture.stderr)?;
    }
    if capture.timed_out {
        eprintln!("sandbox command timed out");
    }
    std::process::exit(capture.exit_code);
}

#[cfg(not(target_os = "windows"))]
fn main() {
    panic!("codex-windows-sandbox-host is Windows-only");
}
