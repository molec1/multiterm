#!/bin/sh
# Build the GTK 3 port. Headers come from the unpacked packages in /tmp/gtk-root.
# The linked sonames are the system libgtk-3 and libvte-2.91, so the binary
# does not need LD_LIBRARY_PATH.
set -eu
. "$HOME/.cargo/env"
export PKG_CONFIG_SYSROOT_DIR=/tmp/gtk-root
export PKG_CONFIG_PATH=/tmp/gtk-root/usr/lib/x86_64-linux-gnu/pkgconfig:/tmp/gtk-root/usr/lib/pkgconfig:/tmp/gtk-root/usr/share/pkgconfig
cd "$(dirname "$0")"
export CARGO_TARGET_DIR="$(pwd)/target"
exec cargo build --release "$@"
