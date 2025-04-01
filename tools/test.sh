#!/bin/sh
set -e
CARGO_WORKSPACE_DIR=$(dirname "$(cargo locate-project --workspace --message-format=plain)")

cargo -Z unstable-options -C $CARGO_WORKSPACE_DIR/tools/koukei run -- -C $CARGO_WORKSPACE_DIR build-image -k "$1"
$CARGO_WORKSPACE_DIR/tools/emulate.sh
