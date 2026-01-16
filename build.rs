use std::{env, path::PathBuf};

fn main() {
    let bindings = bindgen::Builder::default()
        .clang_args(vec!["-x", "c++"])
        .header("wrapper.h")
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let sdk_path = env::var("HIK_SDK_PATH").expect("HIK_SDK_PATH must be set");
    println!("cargo:rustc-link-search={}", sdk_path);
    println!("cargo:rustc-link-lib=hcnetsdk");
}
