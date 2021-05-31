use std::prelude::v1::*;

use std::os::raw::{c_char, c_int};

mod ffi_cow;
pub use ffi_cow::fficow_free;
pub use ffi_cow::{FFICow, FFIStr, FFIString};

mod c_slice;
pub use c_slice::{CSlice, CSliceMut};

use crate::encoding;

#[no_mangle]
pub unsafe extern "C" fn encoding_encode_s(s: *const c_char, len: usize) -> *const FFICow {
    let c_slice = if let Some(c_slice) = CSlice::new(s, len) {
        c_slice
    } else {
        eprintln!("encoding_encode: got a NULL s: {:?}", s);
        return core::ptr::null();
    };

    let cow = encoding::encode(&c_slice);

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
    let c_slice_mut = if let Some(c_slice_mut) = CSliceMut::new_from_size_ptr(buf, bufcap_ptr) {
        c_slice_mut
    } else {
        eprintln!(
            "encoding_encode: got a NULL buf: {:?} or bufcap_ptr: {:?}",
            buf, bufcap_ptr
        );
        return c_int::from(-1);
    };

    let c_slice = if let Some(c_slice) = CSlice::new(s, len) {
        c_slice
    } else {
        eprintln!("encoding_encode: got a NULL s: {:?}", s);
        return c_int::from(-1);
    };

    let bufcap = c_slice_mut.len();

    let bytes = c_slice.as_bytes();
    let min_len = bytes.len();

    eprintln!(
        "encoding_encode: guard ok, bufcap {}, slen {}",
        bufcap, min_len
    );

    if min_len > bufcap {
        eprintln!(
            "encoding_encode: required at least {}, got buffer capacity {}",
            min_len, bufcap
        );
        return c_int::from(-1);
    }

    eprintln!("encoding_encode: encoding");
    let cow = encoding::encode(&c_slice);

    let l = cow.len();
    unsafe { *bufcap_ptr = l + 1 };
    eprintln!(
        "encoding_encode: encoded (len {}/{}): {}",
        l,
        bufcap,
        cow.as_ref(),
    );

    if l >= bufcap {
        eprintln!(
            "encoding_encode: required {}, got capacity {}",
            l + 1,
            bufcap
        );
        return c_int::from(-1);
    }

    let l = match isize::try_from(l) {
        Ok(l) => l,
        Err(_) => return c_int::from(-1),
    };

    eprintln!("Resulting cow: {:?}", cow);

    //let newbuf = if let Cow::Owned(ref r) = cow {
    //    eprintln!("cow is owned: {}", r);
    //    r.as_str().as_ptr()
    //} else {
    //    eprintln!("cow is borrowed: {}", cow.as_ref());
    //    cow.as_ptr()
    //};

    let newbuf = cow.as_bytes();
    eprintln!(
        "encoding_encode: copying buffer from {:?} to {:?}, size: {}\nnewbuf: {:?} {:?} {:?} {:?} {:?} {:?}",
        newbuf, buf, l, newbuf[0], newbuf[1], newbuf[2], newbuf[3], newbuf[4], newbuf[5]
    );

    unsafe {
        core::ptr::copy(newbuf.as_ptr(), buf as *mut _, l as usize);
        *buf.offset(l) = c_char::from(0);
    }
    eprintln!("encoding_encode: done");

    c_int::from(0)
}
