#!/bin/sh
# Run the GTK 3 release binary against the system GTK and VTE libraries.
set -eu
. "$HOME/.cargo/env"
cd "$(dirname "$0")"
exec ./target/release/multiterm "$@"
