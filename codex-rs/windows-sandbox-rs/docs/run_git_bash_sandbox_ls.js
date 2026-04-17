const { spawnSync } = require('node:child_process');
const { existsSync } = require('node:fs');
const { dirname, resolve } = require('node:path');

const repoRoot = resolve(__dirname, '..', '..', '..');
const codexRsDir = resolve(repoRoot, 'codex-rs');
const candidateHostExe = [
  resolve(codexRsDir, 'target', 'release', 'codex-windows-sandbox-host.exe'),
  resolve(codexRsDir, 'target', 'debug', 'codex-windows-sandbox-host.exe'),
];
const hostExe = candidateHostExe.find((p) => existsSync(p));
const codexHome = resolve(repoRoot, '.codex-sandbox-test');

const candidateGitBash = [
  'C:/Program Files/Git/bin/bash.exe',
  'C:/Program Files/Git/usr/bin/bash.exe',
];
const gitFromPathResult = spawnSync('where', ['git.exe'], { encoding: 'utf8' });
const gitBashFromGitExe = gitFromPathResult.status === 0 && gitFromPathResult.stdout
  ? gitFromPathResult.stdout
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((gitExe) => {
        const gitCmdDir = dirname(gitExe);
        const gitRoot = resolve(gitCmdDir, '..');
        return [
          resolve(gitRoot, 'bin', 'bash.exe'),
          resolve(gitRoot, 'usr', 'bin', 'bash.exe'),
        ];
      })
      .flat()
      .find((p) => existsSync(p))
  : null;
const gitBashFromEnv = process.env.GIT_BASH_EXE && existsSync(process.env.GIT_BASH_EXE)
  ? process.env.GIT_BASH_EXE
  : null;
const gitBashFromDefaultPaths = candidateGitBash.find((p) => existsSync(p));
let gitBashFromPath = null;
if (!gitBashFromEnv && !gitBashFromGitExe && !gitBashFromDefaultPaths) {
  const whereResult = spawnSync('where', ['bash.exe'], { encoding: 'utf8' });
  if (whereResult.status === 0 && whereResult.stdout) {
    gitBashFromPath = whereResult.stdout
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line && existsSync(line));
  }
}
const gitBash = gitBashFromEnv ?? gitBashFromGitExe ?? gitBashFromDefaultPaths ?? gitBashFromPath;

if (!hostExe) {
  console.error(`sandbox host not found in: ${candidateHostExe.join(', ')}`);
  process.exit(2);
}
if (!gitBash) {
  console.error(`git bash not found. checked env GIT_BASH_EXE, default paths: ${candidateGitBash.join(', ')}, and PATH via "where bash.exe".`);
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
