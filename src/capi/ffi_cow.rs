use core::mem::ManuallyDrop;
use std::prelude::v1::*;

use std::os::raw::{c_char, c_ulong};

use std::borrow::Cow;

#[derive(Debug)]
#[repr(C)]
pub struct FFIStr {
    len: usize,
    ptr: *const c_char,
}

#[derive(Debug)]
#[repr(C)]
pub struct FFIString {
    len: usize,
    cap: usize,
    ptr: *const c_char,
}

impl Drop for FFIString {
    fn drop(&mut self) {
        let s = unsafe { String::from_raw_parts(self.ptr as *mut _, self.len, self.cap) };
        eprintln!("dropping FFIString {}", &s);
    }
}

#[derive(Debug)]
#[repr(u8, C)]
pub enum FFICow {
    Borrowed(FFIStr),
    Owned(FFIString),
}

impl FFICow {
    pub fn len(&self) -> usize {
        match self {
            Self::Borrowed(FFIStr { len, .. }) | Self::Owned(FFIString { len, .. }) => *len,
        }
    }
}

impl From<Cow<'_, str>> for FFICow {
    fn from(c: Cow<'_, str>) -> Self {
        if let Cow::Owned(s) = c {
            FFICow::Owned(s.into())
        } else {
            FFICow::Borrowed(c.as_ref().into())
        }
    }
}

impl From<FFICow> for Cow<'_, str> {
    fn from(fc: FFICow) -> Self {
        match fc {
            FFICow::Borrowed(b) => {
                let s: &str = From::from(b);
                s.into()
            }
            FFICow::Owned(o) => {
                let s = String::from(o);
                s.into()
            }
        }
    }
}

impl From<FFIStr> for FFICow {
    fn from(fs: FFIStr) -> Self {
        FFICow::Borrowed(fs)
    }
}

impl From<FFIString> for FFICow {
    fn from(fs: FFIString) -> Self {
        FFICow::Owned(fs)
    }
}

impl From<&str> for FFIStr {
    fn from(s: &str) -> Self {
        Self {
            len: s.len(),
            ptr: s.as_ptr() as *const _,
        }
    }
}

impl From<String> for FFIString {
    fn from(s: String) -> Self {
        let s = ManuallyDrop::new(s);
        Self {
            len: s.len(),
            cap: s.capacity(),
            ptr: s.as_ptr() as *const _,
        }
    }
}

impl From<FFIStr> for &str {
    fn from(fs: FFIStr) -> Self {
        let s = unsafe { std::slice::from_raw_parts(fs.ptr as *const _, fs.len) };
        unsafe { std::str::from_utf8_unchecked(s) }
    }
}

impl From<FFIString> for String {
    fn from(fs: FFIString) -> Self {
        // FFIString will be dropped by first converting it to an
        // owned String so we need to ManuallyDrop it.
        let fs = ManuallyDrop::new(fs);
        let s = unsafe { String::from_raw_parts(fs.ptr as *mut _, fs.len, fs.cap) };
        s
    }
}

#[no_mangle]
pub extern "C" fn fficow_len(c: *const FFICow) -> usize {
    eprintln!("called fficow_len: {:?}", c);
    let ffi_cow = unsafe { std::ptr::read::<FFICow>(c) };
    eprintln!("fficow_len: {:?}", ffi_cow);
    let ffi_cow = ManuallyDrop::new(ffi_cow);
    eprintln!("fficow computing len");
    let len = ffi_cow.len();
    eprintln!("fficow len: {}", len);

    //let _len = len.to_ne_bytes();
    //c_ulong::from_ne_bytes(len.to_ne_bytes())
    len
}

#[no_mangle]
pub extern "C" fn fficow_free(c: *const FFICow) {
    let ffi_cow = unsafe { std::ptr::read::<FFICow>(c) };

    eprintln!("freeing a cow: {:?}", ffi_cow);
}
