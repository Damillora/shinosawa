#!/bin/sh
IMAGE_NAME="target/shinosawa.img"
PROFILE=debug

TARGET_ROOTFS=target/rootfs

# Init required directory
mkdir -p $TARGET_ROOTFS

dd if=/dev/zero of=$IMAGE_NAME bs=1M count=100
parted --script $IMAGE_NAME \
mklabel gpt \
mkpart efi fat32 1MiB 100% \
set 1 esp on

# Root required now
sudo losetup /dev/loop0 target/shinosawa.img -P
sudo mkfs.fat -F 32 /dev/loop0p1

# Mount 
sudo mount -t vfat /dev/loop0p1 $TARGET_ROOTFS

# Copy base template
sudo cp -r rootfs/* $TARGET_ROOTFS/

# shinosawa kernel
sudo mkdir -p $TARGET_ROOTFS/shinosawa/system
sudo cp target/x86_64-shinosawa/$PROFILE/kernel $TARGET_ROOTFS/shinosawa/system/

# Clean up
sudo umount $TARGET_ROOTFS
sudo losetup -d /dev/loop0
#mkfs -t ext4 $TARGET_ROOTFS
#mkdir -p $TARGET_ROOTFS
#mount -t auto -o loop  target/shinosawa.img $TARGET_ROOTFS