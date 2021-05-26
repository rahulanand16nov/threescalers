use core::mem::ManuallyDrop;
use std::prelude::v1::*;

use crate::encoding;

//use std::os::raw::c_char;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

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
pub extern "C" fn fficow_free(c: *const FFICow) {
    //let cow: Cow<str> = unsafe { std::ptr::read::<FFICow>(c) }.into();
    let ffi_cow = unsafe { std::ptr::read::<FFICow>(c) };

    eprintln!("freeing a cow: {:?}", ffi_cow);
}

#[no_mangle]
pub extern "C" fn encoding_encode_s(s: *const c_char) -> *const FFICow {
    let s = unsafe { CStr::from_ptr(s) };
    let s = s.to_string_lossy();
    eprintln!("encoding {}", &s);
    let res = encoding::encode(s.as_ref());
    let cow = if let Cow::Owned(r) = res {
        FFICow::Owned(FFIString::from(r))
    } else if let Cow::Owned(s) = s {
        FFICow::Owned(FFIString::from(s))
    } else {
        FFICow::Borrowed(FFIStr::from(res.as_ref()))
    };
    eprintln!("retval ffi_cow {:?}", cow);

    let ret = Box::new(cow);
    eprintln!("retval box<ffi_cow> {:?}", ret);

    let raw = Box::into_raw(ret);
    eprintln!("retval box<ffi_cow>.into_raw {:?}", raw);

    raw as *const _
}

#[no_mangle]
pub extern "C" fn encoding_encode<'a>(s: *const c_char, buf: *mut c_char, len: usize) -> c_int {
    use std::convert::TryFrom;

    let s = unsafe { CStr::from_ptr(s) };
    let s = s.to_string_lossy();
    if s.len() > len {
        return c_int::from(-1);
    }

    let res = encoding::encode(s.as_ref());

    let l = res.len();

    if l >= len {
        return c_int::from(-1);
    }

    let l = match isize::try_from(l) {
        Ok(l) => l,
        Err(_) => return c_int::from(-1),
    };

    let newbuf = if let Cow::Owned(r) = res {
        r.as_ptr()
    } else {
        s.as_ptr()
    };

    unsafe {
        core::ptr::copy(newbuf, buf as *mut _, l as usize);
        *buf.offset(l) = c_char::from(0);
    }

    c_int::from(0)
}
