qemu-system-x86_64 \
        -accel kvm \
		-M q35 \
		-drive if=pflash,unit=0,format=raw,file=ovmf/OVMF_CODE.4m.fd,readonly=on \
		-drive if=pflash,unit=1,format=raw,file=ovmf/OVMF_VARS.4m.fd \
		-hda target/shinosawa.img \
        -m 1G \
		-serial stdio