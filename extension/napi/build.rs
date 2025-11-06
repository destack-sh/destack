fn main() {
    println!("cargo:rerun-if-changed=rust/lib.rs");
    napi_build::setup();
}
