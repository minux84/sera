# Source before building on a host without `nix develop`:  . tools/env.sh
# Puts the pinned pnpm (corepack shim), fontTools venv and webOS NDK in the environment.
# `cd` output is discarded: interactive shells may print the directory (aliases, chpwd hooks).
SERA_ROOT=$(CDPATH= builtin cd -- "$(dirname -- "${BASH_SOURCE[0]:-${(%):-%x}}")/.." >/dev/null 2>&1 && pwd -P)
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0
export WEBOS_NDK="$SERA_ROOT/.build/toolchain/arm-webos-linux-gnueabi_sdk-buildroot"
export PATH="$SERA_ROOT/.build/bin:$SERA_ROOT/.venv/bin:$PATH"
hash -r 2>/dev/null
