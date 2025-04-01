#!/bin/sh
set -e
CARGO_WORKSPACE_DIR=$(dirname "$(cargo locate-project --workspace --message-format=plain)")

qemu-system-x86_64 \
        -accel kvm \
		-M q35 \
		-drive if=pflash,unit=0,format=raw,file=$CARGO_WORKSPACE_DIR/ovmf/OVMF_CODE.4m.fd,readonly=on \
		-drive if=pflash,unit=1,format=raw,file=$CARGO_WORKSPACE_DIR/ovmf/OVMF_VARS.4m.fd \
		-hda $CARGO_WORKSPACE_DIR/target/shinosawa.img \
        -m 1G \
		-serial stdio