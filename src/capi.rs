use std::prelude::v1::*;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

use std::borrow::Cow;

mod ffi_cow;
pub use ffi_cow::fficow_free;
pub use ffi_cow::{FFICow, FFIStr, FFIString};

use crate::encoding;

enum SliceOrCStr<'a> {
    Slice(&'a [u8]),
    CStr(Cow<'a, str>),
}

impl<'a> SliceOrCStr<'a> {
    pub fn new(buf: *const c_char, len: usize) -> Option<Self> {
        if len == 0 {
            SliceOrCStr::CStr(parse_c_str(buf)?)
        } else {
            SliceOrCStr::Slice(parse_buffer(buf as *const _, len)?)
        }
        .into()
    }

    pub fn as_cow(&self) -> Option<Cow<'a, str>> {
        match self {
            Self::Slice(buf) => String::from_utf8_lossy(buf).into(),
            Self::CStr(c) => c.clone(),
        }
        .into()
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Slice(buf) => buf,
            Self::CStr(c) => c.as_bytes(),
        }
    }
}

fn parse_buffer(buf: *const c_void, len: usize) -> Option<&'static [u8]> {
    if buf.is_null() {
        return None;
    }

    unsafe { std::slice::from_raw_parts(buf as *const _, len) }.into()
}

fn parse_buffer_mut(buf: *mut c_void, len: usize) -> Option<&'static mut [u8]> {
    if buf.is_null() {
        return None;
    }

    unsafe { std::slice::from_raw_parts_mut(buf as *mut _, len) }.into()
}

fn parse_c_str(s: *const c_char) -> Option<Cow<'static, str>> {
    if s.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(s) }.to_string_lossy().into()
}

#[no_mangle]
pub unsafe extern "C" fn encoding_encode_s(s: *const c_char, len: usize) -> *const FFICow {
    let slice_or_cstr = if let Some(slice_or_cstr) = SliceOrCStr::new(s, len) {
        slice_or_cstr
    } else {
        eprintln!("encoding_encode: got a NULL s: {:?}", s);
        return core::ptr::null();
    };

    let cow = encoding::encode(slice_or_cstr.as_bytes());

    eprintln!("retval ffi_cow {:?}", cow);

    Box::into_raw(Box::new(cow)) as *const _
}

#[no_mangle]
pub unsafe extern "C" fn encoding_encode(
    s: *const c_char,
    len: usize,
    buf: *mut c_char,
    bufcap_ptr: *mut usize,
) -> c_int {
    use std::convert::TryFrom;

    if buf.is_null() || bufcap_ptr.is_null() {
        eprintln!(
            "encoding_encode: got a NULL buf: {:?}, bufcap_ptr: {:?}",
            buf, bufcap_ptr,
        );
        return c_int::from(-1);
    }

    let slice_or_cstr = if let Some(slice_or_cstr) = SliceOrCStr::new(s, len) {
        slice_or_cstr
    } else {
        eprintln!("encoding_encode: got a NULL s: {:?}", s);
        return c_int::from(-1);
    };

    //eprintln!(
    //    "encoding_encode: ptrs: s: {:?}, len: {}, buf: {:?}, bufcap_ptr: {:?}",
    //    s, len, buf, bufcap_ptr
    //);

    let cap = unsafe { *bufcap_ptr };

    let buffer = unsafe { std::slice::from_raw_parts_mut(buf as *mut _, cap) };

    let bytes = slice_or_cstr.as_bytes();
    let min_len = bytes.len();

    eprintln!(
        "encoding_encode: guard ok, bufcap {}/{}, slen {}",
        cap,
        buffer.len(),
        min_len
    );

    if min_len > cap {
        eprintln!(
            "encoding_encode: required at least {}, got buffer capacity {}",
            min_len, cap
        );
        return c_int::from(-1);
    }

    eprintln!("encoding_encode: encoding");
    let cow = encoding::encode(bytes);

    let l = cow.len();
    unsafe { *bufcap_ptr = l + 1 };
    eprintln!(
        "encoding_encode: encoded (len {}/{}): {}",
        l,
        cap,
        cow.as_ref(),
    );

    if l >= cap {
        eprintln!("encoding_encode: required {}, got capacity {}", l + 1, cap);
        return c_int::from(-1);
    }

    let l = match isize::try_from(l) {
        Ok(l) => l,
        Err(_) => return c_int::from(-1),
    };

    eprintln!("Resulting cow: {:?}", cow);

    let newbuf = if let Cow::Owned(ref r) = cow {
        eprintln!("cow is owned: {}", r);
        r.as_str().as_ptr()
    } else {
        eprintln!("cow is borrowed: {}", cow.as_ref());
        cow.as_ptr()
    };

    eprintln!(
        "encoding_encode: copying buffer from {:?} to {:?}, size: {}",
        newbuf, buf, l
    );
    unsafe {
        eprintln!(
            "newbuf: {:?} {:?} {:?} {:?} {:?} {:?}",
            *newbuf.offset(0),
            *newbuf.offset(1),
            *newbuf.offset(2),
            *newbuf.offset(3),
            *newbuf.offset(4),
            *newbuf.offset(5)
        )
    };
    unsafe {
        core::ptr::copy(newbuf, buf as *mut _, l as usize);
        *buf.offset(l) = c_char::from(0);
    }
    eprintln!("encoding_encode: done");

    c_int::from(0)
}
