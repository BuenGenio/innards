#!/usr/bin/env bash
# Build Innards Linux bundles inside the container. Run from the host:  ~/Projects/innards-build/build.sh
set -euo pipefail
SRC="$HOME/Projects/innards"
IMG=innards-tauri-build
mkdir -p "$HOME/.cargo/registry" "$HOME/.cache/tauri"
docker build -q -t "$IMG" --build-arg UID="$(id -u)" --build-arg GID="$(id -g)" "$(dirname "$0")"
docker run --rm \
  -v "$SRC":/work \
  -v "$HOME/.cargo/registry":/home/build/.cargo/registry \
  -v "$HOME/.cache/tauri":/home/build/.cache/tauri \
  -e CARGO_BUILD_JOBS="$(nproc)" \
  "$IMG" bash -lc 'set -e; pnpm install --frozen-lockfile; pnpm tauri build 2>&1; ls -la target/release/bundle/*/'
