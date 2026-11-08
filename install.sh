#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$project_dir"

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' 'Rust Cargo is required to build. Install Rust, then rerun this script.' >&2
    exit 1
fi

cargo build --release
sudo install -Dm755 target/release/ocypus-a40 /usr/local/bin/ocypus-a40
sudo install -Dm644 udev/99-ocypus-a40.rules /etc/udev/rules.d/99-ocypus-a40.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --action=add --subsystem-match=hidraw
sudo udevadm settle

printf '%s\n' 'Installed ocypus-a40. Run: ocypus-a40 cpu'