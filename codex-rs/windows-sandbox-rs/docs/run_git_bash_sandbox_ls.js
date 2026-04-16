const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { resolve } = require('node:path');

const repoRoot = resolve(__dirname, '..', '..', '..');
const codexRsDir = resolve(repoRoot, 'codex-rs');
const hostExe = resolve(codexRsDir, 'target', 'debug', 'codex-windows-sandbox-host.exe');
const codexHome = resolve(repoRoot, '.codex-sandbox-test');

const candidateGitBash = [
  'C:/Program Files/Git/bin/bash.exe',
  'C:/Program Files/Git/usr/bin/bash.exe',
];
const gitBash = candidateGitBash.find((p) => existsSync(p));

if (!existsSync(hostExe)) {
  console.error(`sandbox host not found: ${hostExe}`);
  process.exit(2);
}
if (!gitBash) {
  console.error('git bash not found in default paths');
  process.exit(3);
}

const sandboxCwd = repoRoot;
const args = [
  '--policy', 'workspace-write',
  '--cwd', sandboxCwd,
  '--codex-home', codexHome,
  '--timeout-ms', '20000',
  '--',
  gitBash,
  '-lc',
  'ls'
];

console.log('Running:', hostExe, args.join(' '));
const result = spawnSync(hostExe, args, {
  cwd: codexRsDir,
  encoding: 'utf8',
});

if (result.stdout) {
  console.log('--- stdout ---');
  process.stdout.write(result.stdout);
}
if (result.stderr) {
  console.log('--- stderr ---');
  process.stderr.write(result.stderr);
}

console.log(`exitCode=${result.status}`);
if (result.error) {
  console.error(result.error);
  process.exit(4);
}
process.exit(result.status ?? 1);
