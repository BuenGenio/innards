# innards-cli

The Innards engine in a terminal: the same probes, findings, capability
scores and upgrade advisor as the desktop app, printed as Markdown or JSON.
It exists for the machines the desktop app can't run on — a phone over
`adb shell` or `ssh`, a Raspberry Pi, a headless box.

```
innards report  [--lang en|es] [--level plain|informed|expert] [--json] [--smart]
innards advise  --uses portable_server,edge_ai [--pains slow,heat_noise]
                [--budget under_300] [--horizon now] [--portable] [--lang es] [--json]
innards socs    [--json]        # the SoC knowledge base (phones, tablets, SBCs)
innards snapshot                # raw probe output, no analysis
```

Nothing leaves the machine. `--smart` is the only thing that may prompt (it
asks for elevated rights to read drive SMART data).

## Build for this machine

```bash
cargo build -p innards-cli --release
./target/release/innards report --level expert
./target/release/innards advise --uses portable_server,edge_ai --budget under_300 --portable
```

## The phone question

"Is my rooted OnePlus 6T any good as a portable server with some on-device
AI?" is exactly what `advise --uses portable_server,edge_ai` answers. Run it
on the phone itself; the engine recognises the SoC (Snapdragon 845 here),
whether it's running Android or a real Linux, root status, UFS storage, the
battery (a free UPS), the missing fan, and Wi-Fi-only networking, then grades
each workload and lists what — if anything — is worth buying.

Two ways to run it there:

### A. On postmarketOS / Mobian / Ubuntu Touch (a real Linux on the phone)

Pick the target by the phone's C library:

| Distro | libc | target |
|---|---|---|
| postmarketOS (Alpine-based) | musl | `aarch64-unknown-linux-musl` (static binary, runs anywhere) |
| Mobian, Ubuntu Touch, Droidian, Armbian, Raspberry Pi OS | glibc | `aarch64-unknown-linux-gnu` |

A `musl` static binary also runs on glibc systems, so when in doubt build
that one. Cross-compile for 64-bit ARM Linux with glibc:

```bash
rustup target add aarch64-unknown-linux-gnu

# A linker for the target. Debian/Ubuntu:
sudo apt install gcc-aarch64-linux-gnu
# Fedora: sudo dnf install gcc-aarch64-linux-gnu
# Arch:   yay -S aarch64-linux-gnu-gcc

# Tell cargo which linker to use (once, in ~/.cargo/config.toml):
cat >> ~/.cargo/config.toml <<'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
EOF

cargo build -p innards-cli --release --target aarch64-unknown-linux-gnu
```

No cross toolchain installed? [`cross`](https://github.com/cross-rs/cross)
does it in a container:

```bash
cargo install cross
cross build -p innards-cli --release --target aarch64-unknown-linux-gnu
```

Then copy and run over SSH (postmarketOS default user is `user`; Mobian is
`mobian`):

```bash
scp target/aarch64-unknown-linux-gnu/release/innards user@<phone-ip>:~/
ssh user@<phone-ip> './innards report --level expert'
ssh user@<phone-ip> './innards advise --uses portable_server,edge_ai --budget under_300 --portable'
```

For postmarketOS (musl) build the fully static binary instead; `cross` is
the simplest way because it brings its own musl toolchain:

```bash
rustup target add aarch64-unknown-linux-musl
cross build -p innards-cli --release --target aarch64-unknown-linux-musl
scp target/aarch64-unknown-linux-musl/release/innards user@<phone-ip>:~/
```

No `cross`, no ARM gcc, but `clang` installed? The musl target is
self-contained (Rust ships the C runtime objects) and Rust ships its own
`lld`, so this links a static aarch64 binary with nothing else:

```bash
rustup target add aarch64-unknown-linux-musl
LLD="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/gcc-ld/ld.lld"
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=clang \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C link-arg=--target=aarch64-unknown-linux-musl -C link-arg=-fuse-ld=$LLD -C link-self-contained=yes -C target-feature=+crt-static" \
cargo build -p innards-cli --release --target aarch64-unknown-linux-musl
file target/aarch64-unknown-linux-musl/release/innards   # ELF 64-bit ... ARM aarch64, statically linked
```

(Use `clang-20` or whatever versioned name your distro installs if plain
`clang` isn't on the path.)

### B. On Android (Termux or `adb shell`)

Build for the Android target with the NDK via
[`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk):

```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk

# Point at an installed NDK (Android Studio's SDK manager, or sdkmanager "ndk;26.3.11579264"):
export ANDROID_NDK_HOME=~/Android/Sdk/ndk/26.3.11579264

cargo ndk -t arm64-v8a build -p innards-cli --release
# → target/aarch64-linux-android/release/innards
```

Run it with `adb`:

```bash
adb push target/aarch64-linux-android/release/innards /data/local/tmp/innards
adb shell chmod +x /data/local/tmp/innards
adb shell /data/local/tmp/innards report --level expert
adb shell /data/local/tmp/innards advise --uses portable_server,edge_ai --budget under_300 --portable
# rooted phones: `adb shell su -c /data/local/tmp/innards report` sees more of /sys
```

Or inside Termux (copy the binary into `$HOME`, not `/sdcard`, which is
mounted `noexec`):

```bash
adb push target/aarch64-linux-android/release/innards /sdcard/innards
# in Termux:
cp /sdcard/innards ~/innards && chmod +x ~/innards && ~/innards report
```

What the Android probe reads: `getprop` (model, platform, Android version),
`/sys/class/power_supply/battery/*`, `/sys/class/thermal/thermal_zone*`,
`/sys/block` (UFS/eMMC), `df -k /data`, and the usual root markers
(`/system/bin/su`, `/system/xbin/su`, `/sbin/su`, `/data/adb/magisk`,
`/data/adb/ksu`). Everything is best-effort: SELinux hides a lot from an
unrooted shell, and the report says what it could not read.

## Output

`report` prints the same Markdown the desktop app exports. `--json` gives
the full structured report (snapshot + findings + capability scores) for
scripting. `advise --json` returns the answers, the raw recommendations and
their rendered text in the chosen language.

Exit codes: 0 on success, 2 on a usage error.
