#!/bin/sh
mkdir target/rootfs
sudo losetup /dev/loop0 target/shinosawa.img
sudo partprobe /dev/loop0
sudo mount /dev/loop0p1 target/rootfs
cd target/rootfs
echo Inspect the rootfs in here! Exiting this shell will clean everything up later.
${SHELL-bash}
cd ../..
sudo umount /dev/loop0p1
sudo losetup -d /dev/loop0
rmdir target/rootfs
