// Install the built IPKs on a rooted TV over root SSH (Homebrew Channel / RootMyTV),
// without an ares developer-mode profile. Same install path Homebrew Channel uses:
// appInstallService/dev/install, then applicationManager/launch.
//
//   node tools/deploy-root.mjs --host 192.168.1.50 [--port 22] [--key ~/.ssh/tv] [--no-player] [--no-launch] [--restart]
//
// Defaults can live in .env.tv (git-ignored): LUMA_TV_HOST, LUMA_TV_PORT, LUMA_TV_USER, LUMA_TV_KEY.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const envFile = path.join(root, '.env.tv');
if (fs.existsSync(envFile)) process.loadEnvFile(envFile);

const args = process.argv.slice(2);
const flag = (name) => args.includes(`--${name}`);
const option = (name, fallback) => {
  const index = args.indexOf(`--${name}`);
  return index >= 0 ? args[index + 1] : fallback;
};
const host = option('host', process.env.LUMA_TV_HOST);
const port = option('port', process.env.LUMA_TV_PORT || '22');
const user = option('user', process.env.LUMA_TV_USER || 'root');
const key = option('key', process.env.LUMA_TV_KEY)?.replace(/^~(?=\/)/, os.homedir());
if (!host || !/^[a-zA-Z0-9.:_-]+$/.test(host))
  throw new Error('Set the TV address: --host <ip> or LUMA_TV_HOST in .env.tv');
if (!/^\d+$/.test(port) || !/^[a-z_][a-z0-9_-]*$/.test(user))
  throw new Error('Invalid --port or --user.');

const read = (file) => JSON.parse(fs.readFileSync(path.join(root, file), 'utf8'));
const app = read('appinfo.json');
const player = read('extras/player/app/appinfo.json');
// --restart: only close and relaunch the installed Luma, no upload or install.
const packages = flag('restart') ? [] : [{ id: app.id, file: `${app.id}_${app.version}_arm.ipk` }];
if (!flag('no-player') && !flag('restart'))
  packages.push({ id: player.id, file: `${player.id}_${player.version}_all.ipk` });
for (const item of packages) {
  item.path = path.join(root, 'dist', item.file);
  if (!fs.existsSync(item.path))
    throw new Error(`Missing ${item.path}. Run pnpm run package first.`);
}

// One multiplexed connection: a password (default "alpine" on many rooted TVs) is asked once.
const control = path.join(os.tmpdir(), `luma-ssh-${process.pid}`);
const sshArgs = [
  '-p',
  port,
  '-o',
  'ControlMaster=auto',
  '-o',
  `ControlPath=${control}`,
  '-o',
  'ControlPersist=120',
  '-o',
  'ConnectTimeout=10',
  '-o',
  'ServerAliveInterval=15',
  ...(key ? ['-i', key] : []),
  `${user}@${host}`,
];
const ssh = (command, stdin = 'inherit') => {
  const result = spawnSync('ssh', [...sshArgs, command], {
    stdio: [stdin, 'inherit', 'inherit'],
    timeout: 600000,
  });
  if (result.error) throw result.error;
  return result.status ?? 1;
};
const must = (status, message) => {
  if (status !== 0) {
    console.error(message);
    process.exit(status || 1);
  }
};

try {
  must(
    ssh('test "$(id -u)" = 0 && command -v luna-send >/dev/null'),
    `SSH to ${user}@${host}:${port} failed, is not root, or is not a webOS TV.`,
  );
  for (const item of packages) {
    const remote = `/tmp/luma-deploy-${item.id}.ipk`;
    const size = (fs.statSync(item.path).size / 1048576).toFixed(1);
    console.log(`Uploading ${item.file} (${size} MiB)…`);
    const fd = fs.openSync(item.path, 'r');
    try {
      must(ssh(`cat > ${remote}`, fd), 'Upload failed.');
    } finally {
      fs.closeSync(fd);
    }
    console.log(`Installing ${item.id}…`);
    // dev/install streams subscription replies; wait for a terminal state.
    const install = `
LOG=/tmp/luma-deploy-${item.id}.log; rm -f "$LOG"
luna-send -w 600000 -i 'luna://com.webos.appInstallService/dev/install' '{"id":"${item.id}","ipkUrl":"${remote}","subscribe":true}' >"$LOG" 2>&1 &
PID=$!; N=0; RC=1
while :; do
  if grep -qE '"state" *: *"installed"' "$LOG"; then RC=0; break; fi
  if grep -qiE '"state" *: *"[^"]*fail|"returnValue" *: *false' "$LOG"; then break; fi
  kill -0 $PID 2>/dev/null || { sleep 1; grep -qE '"state" *: *"installed"' "$LOG" && RC=0; break; }
  N=$((N+1)); [ $N -gt 600 ] && { echo "Install timed out" >&2; break; }
  sleep 1
done
kill $PID 2>/dev/null
[ $RC = 0 ] || cat "$LOG" >&2
rm -f "$LOG" ${remote}
exit $RC`;
    must(ssh(install), `Install of ${item.id} failed.`);
    console.log(`Installed ${item.id}.`);
  }
  if (!flag('no-launch')) {
    // Over a non-interactive SSH session `luna-send -n 1` prints nothing on webOS 10
    // (verified on a 10.3 TV); `-i` into a file and a bounded wait does reply.
    const launch = `
call() {
  F=/tmp/luma-deploy-call.out; rm -f "$F"
  luna-send -w 6000 -i "$1" "$2" >"$F" 2>&1 & P=$!
  N=0; while [ ! -s "$F" ] && [ $N -lt 60 ]; do sleep 0.1; N=$((N+1)); done
  sleep 0.2; kill $P 2>/dev/null; cat "$F"; rm -f "$F"
}
AM=luna://com.webos.service.applicationmanager
call $AM/closeByAppId '{"id":"${app.id}"}' >/dev/null; sleep 1
# Luma restarts its root --pad-watch helper only when the binary is newer than the
# pidfile, but the IPK keeps the build mtime: stop a helper still on the old binary.
W=/tmp/luma-pad-watch.pid; P=$(cat $W 2>/dev/null)
if [ -n "$P" ] && readlink /proc/$P/exe 2>/dev/null | grep -q deleted; then kill $P; rm -f $W; fi
R=$(call $AM/launch '{"id":"${app.id}"}'); echo "launch: $R"
echo "$R" | grep -qE '"returnValue" *: *true' || exit 1
sleep 2; F=$(call $AM/getForegroundAppInfo '{}')
echo "$F" | grep -q '"appId" *: *"${app.id}"' && echo "Luma is in the foreground." || echo "Warning: Luma is not in the foreground: $F"`;
    must(ssh(launch), 'Launch failed.');
  }
} finally {
  spawnSync('ssh', ['-o', `ControlPath=${control}`, '-O', 'exit', `${user}@${host}`], {
    stdio: 'ignore',
  });
}
