use std::sync::OnceLock;

use crate::{LONG, NET_DVR_GetErrorMsg, NET_DVR_GetLastError, NET_DVR_Init, const_ptr_to_string};

static INIT_ONCE: OnceLock<anyhow::Result<()>> = OnceLock::new();

pub fn init() -> &'static anyhow::Result<()> {
    INIT_ONCE.get_or_init(|| {
        unsafe {
            // true is success, false is failed
            let res = NET_DVR_Init();
            if res != 1 {
                return Err(anyhow::anyhow!("Init failed: error code {}", res));
            }
        }
        Ok(())
    })
}

pub fn get_last_error_code() -> i32 {
    unsafe { NET_DVR_GetLastError() as i32 }
}
