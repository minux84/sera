#!/bin/sh
# One-time host setup without Nix (macOS/Homebrew or Linux with the tools preinstalled).
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." >/dev/null 2>&1 && pwd -P)
cd "$ROOT"
if command -v brew >/dev/null; then brew install pkgconf woff2 sdl2; fi
rustup show active-toolchain >/dev/null # installs rust-toolchain.toml (1.93 + ARM target)
python3 -m venv .venv
.venv/bin/pip install -q fonttools brotli
mkdir -p .build/bin
corepack enable --install-directory .build/bin pnpm
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0 PATH="$ROOT/.build/bin:$PATH"
pnpm install --frozen-lockfile
pnpm sdk
echo "Done. Next: . tools/env.sh && pnpm run deploy:root -- --host <TV-IP>"
