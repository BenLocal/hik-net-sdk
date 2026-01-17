# Hikvision Network SDK - Rust Bindings

Rust bindings for the Hikvision Network SDK (HCNetSDK), providing a safe and idiomatic Rust interface to interact with Hikvision network devices.

## Features

- Device login and logout
- Channel information retrieval
- JPEG image capture
- Video file download by time range
- IP channel configuration
- Error handling with detailed error codes

## Requirements

- Rust (stable toolchain)
- Hikvision Network SDK (HCNetSDK) - Windows x64
- Windows OS (currently only Windows is supported)

## Setup

1. Place the Hikvision SDK files in the `sdk/` directory:
   - `HCNetSDK.dll`
   - `HCCore.dll`
   - `HCNetSDKCom/` folder (with all component DLLs)
   - Other required DLLs (see SDK documentation)

2. Set the `HIK_SDK_PATH` environment variable (or use `.cargo/config.toml`):

   ```toml
   [env]
   HIK_SDK_PATH = { value = "sdk", relative = true }
   ```

3. Build the project:

   ```bash
   cargo build
   ```

The build script will automatically copy all required DLLs to the target directory.

## Usage

```rust
use hik_net_sdk::{common, device::HikDevice};

fn main() -> anyhow::Result<()> {
    // Initialize the SDK
    common::init()?;
    
    // Create a device instance
    let mut device = HikDevice::new();
    
    // Login to the device
    device.login("192.168.1.64", "admin", "password", 8000)?;
    
    // Get channel information
    let channels = device.get_channels()?;
    println!("Found {} channels", channels.len());
    
    // Capture a JPEG image
    device.capture_jpeg(1, "output.jpg", 0, 0)?;
    
    // Logout
    device.logout()?;
    
    Ok(())
}
```

## Project Structure

- `src/lib.rs` - Main library entry point and macros
- `src/common.rs` - SDK initialization and common utilities
- `src/device.rs` - Device operations (login, capture, download, etc.)
- `build.rs` - Build script for generating bindings and copying DLLs
- `include/` - C/C++ header files
- `sdk/` - Hikvision SDK DLLs and libraries

## Building

The build process:

1. Generates Rust bindings from C++ headers using `bindgen`
2. Links against `HCNetSDK.lib`
3. Copies all required DLLs to the target directory

## License

This project provides bindings to the Hikvision Network SDK. Please refer to Hikvision's licensing terms for SDK usage.

## Notes

- The SDK must be initialized before use (`common::init()`)
- All DLLs from the SDK directory (including `HCNetSDKCom/`) are automatically copied during build
- The `HCNetSDKCom` folder must be in the same directory as `HCNetSDK.dll` at runtime
