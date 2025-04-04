use std::env;

fn main() {
    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=--image-base=5000000");
}
