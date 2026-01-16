use std::{env, fs, path::PathBuf};

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

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=HCNetSDK");
        copy_sdk(&sdk_path);
    } else {
        println!("cargo:rustc-link-lib=hcnetsdk");
    }
}

fn copy_sdk(sdk_path: &str) {
    let sdk_path = PathBuf::from(sdk_path);
    let target_dir =
        PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let dest_dir = target_dir.join(&profile);
    let deps_dir = dest_dir.join("deps");

    // 复制到两个目录：target/profile 和 target/profile/deps
    // 这样可执行文件和测试可执行文件都能找到DLL
    for dest_base in &[&dest_dir, &deps_dir] {
        println!("cargo:warning=Copying SDK DLLs to {:?}", dest_base);

        // 递归复制所有DLL文件，保持目录结构
        // 这会复制：
        // - 根目录下的所有DLL（HCNetSDK.dll, HCCore.dll, hlog.dll等）
        // - HCNetSDKCom文件夹及其中的所有DLL（文件夹名保持不变）
        copy_dlls_recursive(&sdk_path, dest_base, &sdk_path);
    }

    // 验证必需的DLL是否已复制
    let required_dlls = vec![
        "HCNetSDK.dll",
        "HCCore.dll",
        "HCNetSDKCom", // 文件夹
    ];

    for dest_base in &[&dest_dir, &deps_dir] {
        for item in &required_dlls {
            let dest_path = dest_base.join(item);
            if !dest_path.exists() {
                eprintln!(
                    "cargo:warning=Required SDK file/folder missing: {:?}",
                    dest_path
                );
            }
        }
    }
}

fn copy_dlls_recursive(src_dir: &PathBuf, dest_base: &PathBuf, sdk_root: &PathBuf) {
    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default();

            // 跳过不需要的目录和文件
            let file_name_str = file_name.to_string_lossy();
            if file_name_str == "ClientDemoDll"
                || file_name_str == "ClientDemo.exe"
                || file_name_str == "LocalSensorAdd.dat"
                || file_name_str == "LocalXml.zip"
                || file_name_str.ends_with(".lib")
            {
                continue;
            }

            if path.is_dir() {
                // 递归处理所有目录（包括HCNetSDKCom）
                copy_dlls_recursive(&path, dest_base, sdk_root);
            } else if path.is_file() {
                // 如果是DLL文件，复制它
                if let Some(ext) = path.extension() {
                    if ext == "dll" {
                        // 计算相对路径以保持目录结构
                        let relative_path = path
                            .strip_prefix(sdk_root)
                            .unwrap_or_else(|_| path.file_name().unwrap().as_ref());
                        let dest_path = dest_base.join(relative_path);

                        // 确保目标目录存在
                        if let Some(parent) = dest_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                eprintln!(
                                    "cargo:warning=Failed to create directory {:?}: {}",
                                    parent, e
                                );
                                continue;
                            }
                        }

                        // 复制文件
                        if let Err(e) = fs::copy(&path, &dest_path) {
                            eprintln!(
                                "cargo:warning=Failed to copy {:?} to {:?}: {}",
                                path, dest_path, e
                            );
                        }
                    }
                }
            }
        }
    }
}
