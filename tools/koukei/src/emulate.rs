use std::{
    env,
    process::{Command, Stdio},
};

pub fn command(image: String) {
    // qemu-system-x86_64 \
    // -accel kvm \
    // -M q35 \
    // -drive if=pflash,unit=0,format=raw,file=$CARGO_WORKSPACE_DIR/ovmf/OVMF_CODE.4m.fd,readonly=on \
    // -drive if=pflash,unit=1,format=raw,file=$CARGO_WORKSPACE_DIR/ovmf/OVMF_VARS.4m.fd \
    // -hda $CARGO_WORKSPACE_DIR/target/shinosawa.img \
    // -m 1G \
    // -serial stdio
    println!("using {} as disk image", image);
    
    let exit_status = Command::new("qemu-system-x86_64")
        .envs(env::vars())
        .args([
            "-M",
            "q35,accel=kvm",
            "-cpu","IvyBridge,+x2apic",
            "-drive",
            "if=pflash,unit=0,format=raw,file=ovmf/OVMF_CODE.4m.fd,readonly=on",
            "-drive",
            "if=pflash,unit=1,format=raw,file=ovmf/OVMF_VARS.4m.fd,readonly=on",
            "-hda",
            &image,
            "-m",
            "1G",
            "-serial",
            "stdio",
            "-s",
            // "-d",
            // "int"
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to execute process")
        .wait()
        .unwrap();

    println!("emulator exited with status {}", exit_status.code().unwrap());
}
