use std::prelude::v1::*;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use std::borrow::Cow;

mod ffi_cow;
pub use ffi_cow::fficow_free;
pub use ffi_cow::{FFICow, FFIStr, FFIString};

use crate::encoding;

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

    let raw = Box::into_raw(Box::new(cow));
    eprintln!("retval box<ffi_cow>.into_raw {:?}", raw);

    raw as *const _
}

#[no_mangle]
pub extern "C" fn encoding_encode<'a>(s: *const c_char, buf: *mut c_char, len: usize) -> c_int {
    use std::convert::TryFrom;

    if s.is_null() || buf.is_null() {
        eprintln!("encoding_encode: got a NULL c: {:?}, ptr: {:?}", s, buf);
        return 0;
    }

    eprintln!("encoding_encode: guard ok");
    let s = unsafe { CStr::from_ptr(s) };
    let s = s.to_string_lossy();
    if s.len() > len {
        eprintln!("encoding_encode: required {}, got len {}", s.len(), len);
        return c_int::from(-1);
    }

    eprintln!("encoding_encode: encoding");
    let res = encoding::encode(s.as_ref());

    let l = res.len();
    eprintln!(
        "encoding_encode: encoded (len {}/{}): {}",
        l,
        len,
        res.as_ref()
    );

    if l >= len {
        eprintln!("encoding_encode: required {}, got len {}", l, len);
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

    eprintln!("encoding_encode: copying buffer");
    unsafe {
        core::ptr::copy(newbuf, buf as *mut _, l as usize);
        *buf.offset(l) = c_char::from(0);
    }
    eprintln!("encoding_encode: done");

    c_int::from(0)
}
