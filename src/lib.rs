#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub mod common;
pub mod device;

#[macro_export]
macro_rules! as_c_string {
    ($a:ident) => {
        std::ffi::CString::new($a).unwrap()
    };
    ($a:expr) => {
        std::ffi::CString::new($a).unwrap()
    };
}

#[macro_export]
macro_rules! const_ptr_to_string {
    ($a:ident) => {
        unsafe { std::ffi::CStr::from_ptr($a).to_string_lossy().into_owned() }
    };
    ($a:expr) => {
        unsafe { std::ffi::CStr::from_ptr($a).to_string_lossy().into_owned() }
    };
    ($a:ident, $def:literal) => {
        if $a.is_null() {
            $def
        } else {
            unsafe { std::ffi::CStr::from_ptr($a).to_string_lossy().into_owned() }
        }
    };
    ($a:expr, $def:expr) => {{
        let ptr = $a as *const std::os::raw::c_char;
        if ptr.is_null() {
            $def
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| $def)
            }
        }
    }};
}
