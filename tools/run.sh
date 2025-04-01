#!/bin/sh
set -e
CARGO_WORKSPACE_DIR=$(dirname "$(cargo locate-project --workspace --message-format=plain)")

if [[ -n "$1" ]] then
    cargo -Z unstable-options -C $CARGO_WORKSPACE_DIR/tools/koukei run -- -C $CARGO_WORKSPACE_DIR build-image -k "$1"
else
    cargo -Z unstable-options -C $CARGO_WORKSPACE_DIR/tools/koukei run -- -C $CARGO_WORKSPACE_DIR build-image
fi
cargo -Z unstable-options -C $CARGO_WORKSPACE_DIR/tools/koukei run -- -C $CARGO_WORKSPACE_DIR emulate
