#!/bin/sh
set -eu

workspace_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
binary="$workspace_root/target/thumbv6m-none-eabi/release/rmk-ht32-60key"
llvm_size=$(find "$(rustc --print sysroot)" -type f -name llvm-size | head -n 1)

if [ ! -x "$llvm_size" ]; then
    echo "llvm-size not found; install the llvm-tools-preview rustup component" >&2
    exit 2
fi

# Always rebuild the package in its own Cargo invocation so features enabled by
# a preceding workspace-wide build cannot change the measured image.
cargo build --manifest-path "$workspace_root/Cargo.toml" --locked --release -p rmk-ht32-60key

sections=$($llvm_size -A "$binary")
flash=$(printf '%s\n' "$sections" | awk '$1 == ".vector_table" || $1 == ".text" || $1 == ".rodata" || $1 == ".data" { total += $2 } END { print total + 0 }')
static_ram=$(printf '%s\n' "$sections" | awk '$1 == ".data" || $1 == ".bss" || $1 == ".uninit" { total += $2 } END { print total + 0 }')
ram_headroom=$((16384 - static_ram))

printf 'Flash: %s / 130560 bytes\n' "$flash"
printf 'Static RAM: %s / 16384 bytes\n' "$static_ram"
printf 'RAM headroom: %s bytes\n' "$ram_headroom"

if [ "$flash" -gt 130560 ]; then
    echo "error: application Flash budget exceeded" >&2
    exit 1
fi

if [ "$ram_headroom" -lt 2048 ]; then
    echo "error: less than 2048 bytes remain for stack and runtime use" >&2
    exit 1
fi
