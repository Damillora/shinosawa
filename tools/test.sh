#!/bin/sh
set -e
CARGO_WORKSPACE_DIR=$(dirname "$(cargo locate-project --workspace --message-format=plain)")

$CARGO_WORKSPACE_DIR/tools/build_image.sh "$1"
$CARGO_WORKSPACE_DIR/tools/emulate.sh
