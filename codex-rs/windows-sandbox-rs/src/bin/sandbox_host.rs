#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    use codex_windows_sandbox::ElevatedSandboxCaptureRequest;
    use codex_windows_sandbox::NetworkMode;
    use codex_windows_sandbox::parse_policy;
    use codex_windows_sandbox::run_windows_sandbox_capture_elevated;
    use codex_windows_sandbox::run_windows_sandbox_capture_with_extra_deny_write_paths;
    use codex_windows_sandbox::sandbox_setup_is_complete;
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::PathBuf;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Backend {
        Auto,
        Elevated,
        Unelevated,
    }

    fn path_eq(left: &std::path::Path, right: &std::path::Path) -> bool {
        left.to_string_lossy()
            .eq_ignore_ascii_case(right.to_string_lossy().as_ref())
    }

    let mut args = std::env::args().skip(1);
    let mut policy = String::from("read-only");
    let mut backend = Backend::Auto;
    let mut policy_cwd: Option<PathBuf> = None;
    let mut cwd: Option<PathBuf> = None;
    let mut codex_home: Option<PathBuf> = None;
    let mut timeout_ms: Option<u64> = None;
    let mut use_private_desktop = false;
    let mut proxy_enforced = false;
    let mut network_mode = NetworkMode::Default;
    let mut read_roots: Vec<PathBuf> = Vec::new();
    let mut write_roots: Vec<PathBuf> = Vec::new();
    let mut deny_read_paths: Vec<PathBuf> = Vec::new();
    let mut deny_write_paths: Vec<PathBuf> = Vec::new();
    let mut temp_root: Option<PathBuf> = None;
    let mut env_map: HashMap<String, String> = std::env::vars().collect();
    let mut command: Vec<String> = Vec::new();
    let mut print_capabilities_json = false;
    let mut print_probe_json = false;

    const HELP_TEXT: &str = "\
codex-windows-sandbox-host - run a command through Windows sandbox setup + runner

Usage:
  codex-windows-sandbox-host [OPTIONS] -- <COMMAND> [ARGS...]

Examples:
  codex-windows-sandbox-host --backend auto --policy workspace-write -- cmd /c \"echo HOST_OK\"
  codex-windows-sandbox-host --backend unelevated --policy workspace-write -- cmd /c \"echo HOST_OK\"
  codex-windows-sandbox-host --backend elevated --policy workspace-write -- cmd /c \"echo HOST_OK\"
  codex-windows-sandbox-host --policy workspace-write -- cmd /c \"echo HOST_OK\"
  codex-windows-sandbox-host --clear-env --env PATH=C:\\Windows\\System32 -- cmd /c ver
  codex-windows-sandbox-host --policy-cwd C:\\work --policy \"{\\\"type\\\":\\\"read-only\\\"}\" -- powershell -NoProfile -Command Get-ChildItem

Options:
  -h, --help
      Show this help text.

  --capabilities --json
      Print passive host capability JSON and exit.

  --probe --json
      Print host capability JSON. Active probes are not run by this command yet.

  --policy <read-only|workspace-write|JSON>
      Sandbox policy preset or raw SandboxPolicy JSON.
      Default: read-only.
      Notes:
      - danger-full-access and external-sandbox are rejected.
      - JSON is parsed exactly as codex_protocol::protocol::SandboxPolicy.

  --backend <auto|elevated|unelevated>
      Select Windows sandbox backend.
      Default: auto.
      auto selection rules:
      - Use elevated when --proxy-enforced is set.
      - Use elevated when --read-root or --write-root is provided.
      - Use elevated when policy requires restricted read access.
      - Otherwise use elevated only if sandbox setup marker already exists.
      - Fallback to unelevated when setup is not complete.

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
      Supported only by elevated backend.

  --read-root <PATH>      (repeatable)
      Override computed readable roots with an explicit root list.
      When present at least once, only the provided values are used.
      Supported only by elevated backend.

  --write-root <PATH>     (repeatable)
      Override computed writable roots with an explicit root list.
      When present at least once, only the provided values are used.
      Supported only by elevated backend.

  --deny-write-path <PATH> (repeatable)
      Add explicit deny-write paths that remain read-only even under
      workspace-write configurations.

  --deny-read-path <PATH> (repeatable)
      Add explicit deny-read paths that remain unreadable even under explicit
      read roots. Supported only by elevated backend.

  --temp-root <PATH>
      Use PATH as the child TEMP/TMP directory and add it to writable roots.
      Host TEMP/TMP are not made writable unless they are this same path.

  --network <none|default>
      Select host network behavior.
      default follows the sandbox policy. none forces the offline identity.

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
            "--capabilities" => {
                print_capabilities_json = true;
            }
            "--probe" => {
                print_probe_json = true;
            }
            "--json" => {}
            "--policy" => {
                policy = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for --policy"))?;
            }
            "--backend" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for --backend"))?;
                backend = match value.as_str() {
                    "auto" => Backend::Auto,
                    "elevated" => Backend::Elevated,
                    "unelevated" => Backend::Unelevated,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "invalid --backend: {value}. Expected auto|elevated|unelevated"
                        ));
                    }
                };
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
            "--network" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for --network"))?;
                network_mode = match value.as_str() {
                    "default" => NetworkMode::Default,
                    "none" => NetworkMode::None,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "invalid --network: {value}. Expected none|default"
                        ));
                    }
                };
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
            "--deny-read-path" => {
                deny_read_paths
                    .push(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --deny-read-path")
                    })?));
            }
            "--deny-write-path" => {
                deny_write_paths
                    .push(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --deny-write-path")
                    })?));
            }
            "--temp-root" => {
                temp_root =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("missing value for --temp-root")
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

    if print_capabilities_json || print_probe_json {
        println!(
            "{{\"version\":1,\"backend\":\"auto\",\"fullSandbox\":true,\"explicitRoots\":true,\"denyRead\":true,\"denyWrite\":true,\"tempRoot\":true,\"networkIsolation\":true,\"jobObjectKillTree\":true,\"clearEnv\":true,\"limitedReasons\":[]}}"
        );
        return Ok(());
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
    if let Some(temp_root) = temp_root.as_ref() {
        std::fs::create_dir_all(temp_root)?;
        let temp_root = dunce::canonicalize(temp_root).unwrap_or_else(|_| temp_root.clone());
        env_map.insert("TEMP".to_string(), temp_root.to_string_lossy().to_string());
        env_map.insert("TMP".to_string(), temp_root.to_string_lossy().to_string());
        if !write_roots
            .iter()
            .any(|root| path_eq(root.as_path(), temp_root.as_path()))
        {
            write_roots.push(temp_root);
        }
    }

    let write_roots_override = if write_roots.is_empty() {
        None
    } else {
        Some(write_roots.as_slice())
    };

    let resolved_backend = match backend {
        Backend::Elevated => Backend::Elevated,
        Backend::Unelevated => Backend::Unelevated,
        Backend::Auto => {
            let parsed_policy = parse_policy(policy.as_str())?;
            let needs_elevated = proxy_enforced
                || read_roots_override.is_some()
                || write_roots_override.is_some()
                || !deny_read_paths.is_empty()
                || !deny_write_paths.is_empty()
                || temp_root.is_some()
                || matches!(network_mode, NetworkMode::None)
                || !parsed_policy.has_full_disk_read_access();
            if needs_elevated || sandbox_setup_is_complete(codex_home.as_path()) {
                Backend::Elevated
            } else {
                Backend::Unelevated
            }
        }
    };

    let capture = match resolved_backend {
        Backend::Elevated => run_windows_sandbox_capture_elevated(ElevatedSandboxCaptureRequest {
            policy_json_or_preset: policy.as_str(),
            sandbox_policy_cwd: policy_cwd.as_path(),
            codex_home: codex_home.as_path(),
            command,
            cwd: cwd.as_path(),
            env_map,
            timeout_ms,
            use_private_desktop,
            proxy_enforced,
            network_mode,
            read_roots_override,
            read_roots_include_platform_defaults: false,
            write_roots_override,
            deny_read_paths_override: deny_read_paths.as_slice(),
            deny_write_paths_override: deny_write_paths.as_slice(),
        })?,
        Backend::Unelevated => {
            if proxy_enforced {
                return Err(anyhow::anyhow!(
                    "--proxy-enforced is only supported with --backend elevated (or auto)"
                ));
            }
            if read_roots_override.is_some()
                || write_roots_override.is_some()
                || !deny_read_paths.is_empty()
                || temp_root.is_some()
                || matches!(network_mode, NetworkMode::None)
            {
                return Err(anyhow::anyhow!(
                    "--read-root/--write-root/--deny-read-path/--temp-root/--network none are only supported with --backend elevated (or auto)"
                ));
            }
            run_windows_sandbox_capture_with_extra_deny_write_paths(
                policy.as_str(),
                policy_cwd.as_path(),
                codex_home.as_path(),
                command,
                cwd.as_path(),
                env_map,
                timeout_ms,
                deny_write_paths.as_slice(),
                use_private_desktop,
            )?
        }
        Backend::Auto => unreachable!("backend auto must be resolved before execution"),
    };

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
