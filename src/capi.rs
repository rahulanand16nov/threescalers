use core::mem::ManuallyDrop;
use std::prelude::v1::*;

use crate::encoding;

//use std::os::raw::c_char;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use std::borrow::Cow;

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
    Borrowed(*const c_char),
    Owned(FFIString),
}

impl From<Cow<'_, str>> for FFICow {
    fn from(c: Cow<'_, str>) -> Self {
        if let Cow::Owned(s) = c {
            FFICow::Owned(s.into())
        } else {
            FFICow::Borrowed(c.as_ref().as_ptr() as *const _)
        }
    }
}

impl From<FFICow> for Cow<'_, str> {
    fn from(fc: FFICow) -> Self {
        match fc {
            FFICow::Borrowed(c) => {
                let s = unsafe { std::ffi::CStr::from_ptr(c) }.to_string_lossy();
                s
            }
            FFICow::Owned(o) => {
                let s = String::from(o);
                s.into()
            }
        }
    }
}

impl From<FFIString> for FFICow {
    fn from(fs: FFIString) -> Self {
        FFICow::Owned(fs)
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
        FFICow::Borrowed(res.as_ptr() as *const _)
    };

    Box::into_raw(Box::new(cow)) as *const _
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
