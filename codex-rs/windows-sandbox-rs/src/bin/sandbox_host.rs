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

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => {
                command.extend(args);
                break;
            }
            "-h" | "--help" => {
                println!(
                    "Usage: codex-windows-sandbox-host [OPTIONS] -- <COMMAND> [ARGS...]\n\n\
                     Options:\n\
                     --policy <read-only|workspace-write|JSON>\n\
                     --policy-cwd <PATH>\n\
                     --cwd <PATH>\n\
                     --codex-home <PATH>\n\
                     --timeout-ms <U64>\n\
                     --private-desktop\n\
                     --proxy-enforced\n\
                     --read-root <PATH> (repeatable)\n\
                     --write-root <PATH> (repeatable)\n\
                     --deny-write-path <PATH> (repeatable)\n\
                     --env <KEY=VALUE> (repeatable)\n\
                     --clear-env"
                );
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
