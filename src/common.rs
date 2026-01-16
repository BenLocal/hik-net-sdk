use std::sync::OnceLock;

use crate::{NET_DVR_GetLastError, NET_DVR_Init};

static INIT_ONCE: OnceLock<Result<(), i32>> = OnceLock::new();

pub fn init() -> anyhow::Result<()> {
    let result = INIT_ONCE.get_or_init(|| {
        unsafe {
            // true is success, false is failed
            let res = NET_DVR_Init();
            if res != 1 {
                return Err(res);
            }
        }
        Ok(())
    });
    
    match result {
        Ok(()) => Ok(()),
        Err(code) => Err(anyhow::anyhow!("Init failed: error code {}", code)),
    }
}

pub fn get_last_error_code() -> i32 {
    unsafe { NET_DVR_GetLastError() as i32 }
}
