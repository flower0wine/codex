const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const { resolve, join } = require('node:path');

const repoRoot = resolve(__dirname, '..', '..', '..');
const codexRsDir = resolve(repoRoot, 'codex-rs');
const candidateHostExe = [
  resolve(codexRsDir, 'target', 'release', 'codex-windows-sandbox-host.exe'),
  resolve(codexRsDir, 'target', 'debug', 'codex-windows-sandbox-host.exe'),
];
const hostExe = candidateHostExe.find((p) => fs.existsSync(p));
const codexHome = resolve(repoRoot, '.codex-sandbox-test');

const testRoot = resolve(codexRsDir, 'windows-sandbox-rs', 'sandbox-feature-tests');
const ws = join(testRoot, 'ws');
const outside = join(testRoot, 'outside');
const deny = join(ws, 'deny');
const reportPath = join(testRoot, 'last-supported-shell-report.json');

if (!hostExe) {
  console.error(`sandbox host not found in: ${candidateHostExe.join(', ')}`);
  process.exit(2);
}

fs.mkdirSync(ws, { recursive: true });
fs.mkdirSync(outside, { recursive: true });
fs.mkdirSync(deny, { recursive: true });

for (const dir of [ws, outside, deny]) {
  for (const item of fs.readdirSync(dir, { withFileTypes: true })) {
    if (item.isFile()) {
      fs.rmSync(join(dir, item.name), { force: true });
    }
  }
}

function runHost(args) {
  const result = spawnSync(hostExe, args, {
    cwd: codexRsDir,
    encoding: 'utf8',
    maxBuffer: 10 * 1024 * 1024,
  });
  return {
    code: result.status ?? -1,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    error: result.error ? String(result.error) : '',
    args,
  };
}

function combineOutput(r) {
  return `${r.stdout}${r.stderr}${r.error}`.trim();
}

function addResult(results, name, required, pass, runResult, note = '') {
  results.push({
    name,
    required,
    pass,
    exitCode: runResult.code,
    note,
    detail: combineOutput(runResult),
    args: runResult.args,
  });
}

function psSingleQuoted(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

const results = [];

const baseArgs = ['--cwd', ws, '--codex-home', codexHome];

let r = runHost(['--help']);
addResult(
  results,
  'help_text',
  true,
  r.code === 0 && /Usage:/i.test(r.stdout),
  r
);

r = runHost(['--policy', 'workspace-write', ...baseArgs, '--', 'cmd', '/c', 'echo CMD_OK']);
addResult(results, 'cmd_basic', true, r.code === 0 && r.stdout.includes('CMD_OK'), r);

r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'powershell',
  '-NoLogo',
  '-NoProfile',
  '-Command',
  'Write-Output PS_OK',
]);
addResult(results, 'powershell_basic', true, r.code === 0 && r.stdout.includes('PS_OK'), r);

const wsOkFile = join(ws, 'ws_ok.txt');
fs.rmSync(wsOkFile, { force: true });
r = runHost(['--policy', 'workspace-write', ...baseArgs, '--', 'cmd', '/c', 'echo OK>ws_ok.txt']);
addResult(
  results,
  'workspace_write_in_cwd',
  true,
  r.code === 0 && fs.existsSync(wsOkFile),
  r
);

const roFailFile = join(ws, 'ro_fail.txt');
fs.rmSync(roFailFile, { force: true });
r = runHost(['--policy', 'read-only', ...baseArgs, '--', 'cmd', '/c', 'echo NO>ro_fail.txt']);
addResult(
  results,
  'readonly_write_denied',
  true,
  r.code !== 0 && !fs.existsSync(roFailFile),
  r
);

const outsideFailFile = join(outside, 'outside_fail.txt');
fs.rmSync(outsideFailFile, { force: true });
r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'cmd',
  '/c',
  `echo NO>${outsideFailFile}`,
]);
addResult(
  results,
  'workspace_outside_write_denied',
  true,
  r.code !== 0 && !fs.existsSync(outsideFailFile),
  r
);

r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--clear-env',
  '--env',
  'FOO=BAR',
  '--',
  'cmd',
  '/c',
  'echo %FOO%',
]);
addResult(results, 'env_clear_and_set', true, r.code === 0 && r.stdout.trim() === 'BAR', r);

r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--timeout-ms',
  '1000',
  '--',
  'powershell',
  '-NoLogo',
  '-NoProfile',
  '-Command',
  'Start-Sleep -Seconds 3',
]);
addResult(results, 'timeout_exit_192', true, r.code === 192, r);

r = runHost([
  '--backend',
  'unelevated',
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'cmd',
  '/c',
  'echo UNELEVATED_OK',
]);
addResult(
  results,
  'backend_unelevated',
  true,
  r.code === 0 && r.stdout.includes('UNELEVATED_OK'),
  r
);

r = runHost([
  '--backend',
  'elevated',
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'cmd',
  '/c',
  'echo ELEVATED_OK',
]);
addResult(
  results,
  'backend_elevated',
  true,
  r.code === 0 && r.stdout.includes('ELEVATED_OK'),
  r
);

const rwOverrideFile = join(ws, 'rw_override.txt');
fs.rmSync(rwOverrideFile, { force: true });
r = runHost([
  '--backend',
  'elevated',
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--read-root',
  ws,
  '--write-root',
  ws,
  '--',
  'cmd',
  '/c',
  'echo RW_OK>rw_override.txt',
]);
addResult(
  results,
  'elevated_read_write_root_override',
  true,
  r.code === 0 && fs.existsSync(rwOverrideFile),
  r
);

const denyFailFile = join(deny, 'deny_fail.txt');
fs.rmSync(denyFailFile, { force: true });
r = runHost([
  '--backend',
  'elevated',
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--deny-write-path',
  deny,
  '--',
  'cmd',
  '/c',
  'echo NO>deny\\deny_fail.txt',
]);
addResult(
  results,
  'deny_write_path_enforced',
  true,
  r.code !== 0 && !fs.existsSync(denyFailFile),
  r
);

r = runHost([
  '--backend',
  'elevated',
  '--private-desktop',
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'cmd',
  '/c',
  'echo PRIVATE_DESKTOP_OK',
]);
addResult(
  results,
  'private_desktop_flag',
  true,
  r.code === 0 && r.stdout.includes('PRIVATE_DESKTOP_OK'),
  r
);

const networkProbe = [
  "$ProgressPreference = 'SilentlyContinue'",
  'try {',
  "  $response = Invoke-WebRequest -Uri 'https://www.example.com/' -UseBasicParsing -TimeoutSec 8",
  '  if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 500) {',
  "    Write-Output 'NETWORK_OK'",
  '    exit 0',
  '  }',
  "  Write-Output ('NETWORK_STATUS_' + $response.StatusCode)",
  '  exit 2',
  '} catch {',
  '  Write-Error $_',
  '  exit 1',
  '}',
].join('; ');
r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--timeout-ms',
  '15000',
  '--',
  'powershell',
  '-NoLogo',
  '-NoProfile',
  '-Command',
  networkProbe,
]);
addResult(
  results,
  'diagnostic_network_reachable',
  false,
  r.code === 0 && r.stdout.includes('NETWORK_OK'),
  r,
  'Diagnostic only: pass means outbound HTTPS was reachable from inside the sandbox'
);

const outsideScript = join(outside, 'outside_script.ps1');
fs.writeFileSync(outsideScript, "Write-Output 'OUTSIDE_SCRIPT_OK'\n", 'utf8');
r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'powershell',
  '-NoLogo',
  '-NoProfile',
  '-ExecutionPolicy',
  'Bypass',
  '-File',
  outsideScript,
]);
addResult(
  results,
  'diagnostic_execute_script_outside_cwd',
  false,
  r.code === 0 && r.stdout.includes('OUTSIDE_SCRIPT_OK'),
  r,
  'Diagnostic only: pass means a script located outside the workspace CWD was executable'
);

const outsideFromScriptFile = join(outside, 'outside_from_workspace_script.txt');
const workspaceWriterScript = join(ws, 'write_outside.ps1');
fs.rmSync(outsideFromScriptFile, { force: true });
fs.writeFileSync(
  workspaceWriterScript,
  [
    "$ErrorActionPreference = 'Stop'",
    `$path = ${psSingleQuoted(outsideFromScriptFile)}`,
    "Set-Content -LiteralPath $path -Value 'OUTSIDE_WRITE_OK'",
    "Write-Output 'WORKSPACE_SCRIPT_OUTSIDE_WRITE_OK'",
  ].join('\n') + '\n',
  'utf8'
);
r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'powershell',
  '-NoLogo',
  '-NoProfile',
  '-ExecutionPolicy',
  'Bypass',
  '-File',
  workspaceWriterScript,
]);
addResult(
  results,
  'diagnostic_workspace_script_writes_outside_cwd',
  false,
  r.code === 0 && fs.existsSync(outsideFromScriptFile),
  r,
  'Diagnostic only: pass means a workspace script was able to create a file outside the workspace CWD'
);

r = runHost([
  '--policy',
  'workspace-write',
  ...baseArgs,
  '--',
  'pwsh',
  '-NoLogo',
  '-NoProfile',
  '-Command',
  'Write-Output PWSH_OK',
]);
addResult(
  results,
  'pwsh_optional',
  false,
  r.code === 0 && r.stdout.includes('PWSH_OK'),
  r,
  'Optional capability'
);

const required = results.filter((x) => x.required);
const passedRequired = required.filter((x) => x.pass).length;
const failedRequired = required.length - passedRequired;
const optional = results.filter((x) => !x.required);
const passedOptional = optional.filter((x) => x.pass).length;

const report = {
  hostExe,
  codexHome,
  workspace: ws,
  generatedAt: new Date().toISOString(),
  summary: {
    total: results.length,
    requiredTotal: required.length,
    requiredPassed: passedRequired,
    requiredFailed: failedRequired,
    optionalTotal: optional.length,
    optionalPassed: passedOptional,
  },
  results,
};

fs.mkdirSync(testRoot, { recursive: true });
fs.writeFileSync(reportPath, JSON.stringify(report, null, 2), 'utf8');

console.log(`Report: ${reportPath}`);
console.log(
  `Required: ${passedRequired}/${required.length} passed` +
    (optional.length > 0 ? ` | Optional: ${passedOptional}/${optional.length} passed` : '')
);
for (const item of results) {
  const label = item.pass ? 'PASS' : 'FAIL';
  const flag = item.required ? 'required' : 'optional';
  console.log(`[${label}] (${flag}) ${item.name} (exit=${item.exitCode})`);
}

process.exit(failedRequired === 0 ? 0 : 1);
