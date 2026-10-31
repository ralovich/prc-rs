// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use crate::common::{ParsingOptions, prc_describe};

/// Function pointer to caller defined memory allocator. Memory allocated
/// by this function needs to be released by the caller with a matching
/// deallocator. The returned allocation should be at least 16 byte aligned.
/// Part of the public C API.
pub type AllocateFnT = ::std::option::Option<unsafe extern "C" fn(argument: u64) -> *mut u8>;

/// Return textual representation of a parsed PRC.
///
/// \param[in] src_len Number of bytes in PRC data.
/// \param[in] src Pointer to PRC data.
/// \param[in] allocate function pointer to caller defined memory allocator.
/// \param[out] dst Address of pointer to data being returned. Allocated by \c allocate.
/// \param[out] dst_actual_size Number of bytes used in \c dst.
/// \param reserved Reserved for future use.
/// \returns 0 on success.
/// Part of the public C API.
/// # Safety
///
/// Use of raw pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn prc_parse_to_json(
    src_len: u64,
    src: *const u8,
    allocate: AllocateFnT,
    dst: *mut *mut u8,
    dst_actual_size: *mut u64,
    _reserved: *mut u64,
) -> i32 {
    if src_len < 1 {
        return -1;
    }
    if src.is_null() {
        return -2;
    }
    if allocate.is_none() {
        return -3;
    }
    if dst.is_null() {
        return -4;
    }
    if dst_actual_size.is_null() {
        return -5;
    }

    let src_slice = unsafe { std::slice::from_raw_parts(src, src_len as usize) };

    let opts: ParsingOptions = ParsingOptions {
        verbose: true,
        all: true,
        globals: true,
        tree: true,
        tess: true,
        geom: true,
        extgeom: true,
        schema: true,
        modelfile: true,
        dump_attachments: false,
    };
    let rv = prc_describe(src_slice, "capi.prc_parse_to_json.prc", &opts);
    match rv {
        Err(_) => {
            unsafe {
                *dst_actual_size = 0;
            }
            -10
        }
        Ok(parsed_prc) => {
            // copy resulting text into dst
            let ser = serde_json::to_string(&parsed_prc);
            if let Ok(parsed) = ser {
                let parsed = parsed.as_bytes();

                let allocate_func = allocate.unwrap();
                unsafe {
                    *dst = allocate_func(parsed.len() as u64);
                    if (*dst).is_null() {
                        return -12;
                    }
                }
                // copy resulting text into dst
                unsafe {
                    std::ptr::copy_nonoverlapping::<u8>(&parsed[0], *dst, parsed.len());
                    *dst_actual_size = parsed.len() as u64;
                }
                0
            } else {
                -11
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trim_trailing_newline(s: &mut String) -> &String {
        if s.ends_with('\n') {
            s.pop();
        }
        if s.ends_with('\r') {
            s.pop();
        }
        s
    }

    #[test]
    fn test_c_api() {
        let bytes_external = std::fs::read(std::string::String::from(
            "testdata/pmi_sample.stream-23.prc",
        ))
        .unwrap();
        assert_eq!(bytes_external.len(), 24535usize);
        let mut json_expected = std::fs::read_to_string(std::string::String::from(
            "testdata/pmi_sample.stream-23.json",
        ))
        .unwrap();
        // The C++ prc2json tool ends the output with a newline.
        trim_trailing_newline(&mut json_expected);

        let mut dst: *mut u8 = std::ptr::null_mut();
        let mut dst_actual_size: u64 = 0;
        let reserved: *mut u64 = std::ptr::null_mut();

        unsafe extern "C" fn allocate_fn(num_bytes: u64) -> *mut u8 {
            let layout = std::alloc::Layout::from_size_align(num_bytes as usize, 16).unwrap();
            unsafe { std::alloc::alloc(layout) }
        }

        unsafe {
            let rv = prc_parse_to_json(
                0,
                &bytes_external[0],
                Some(allocate_fn),
                &mut dst,
                &mut dst_actual_size,
                reserved,
            );
            assert_ne!(rv, 0);
        }
        unsafe {
            let rv = prc_parse_to_json(
                bytes_external.len() as u64,
                std::ptr::null(),
                Some(allocate_fn),
                &mut dst,
                &mut dst_actual_size,
                reserved,
            );
            assert_ne!(rv, 0);
        }
        unsafe {
            let rv = prc_parse_to_json(
                bytes_external.len() as u64,
                &bytes_external[0],
                None,
                &mut dst,
                &mut dst_actual_size,
                reserved,
            );
            assert_ne!(rv, 0);
        }
        unsafe {
            let rv = prc_parse_to_json(
                bytes_external.len() as u64,
                &bytes_external[0],
                Some(allocate_fn),
                std::ptr::null_mut(),
                &mut dst_actual_size,
                reserved,
            );
            assert_ne!(rv, 0);
        }
        unsafe {
            let rv = prc_parse_to_json(
                bytes_external.len() as u64,
                &bytes_external[0],
                Some(allocate_fn),
                &mut dst,
                std::ptr::null_mut(),
                reserved,
            );
            assert_ne!(rv, 0);
        }

        unsafe {
            let rv = prc_parse_to_json(
                bytes_external.len() as u64,
                &bytes_external[0],
                Some(allocate_fn),
                &mut dst,
                &mut dst_actual_size,
                reserved,
            );
            assert_eq!(rv, 0);

            assert!(!dst.is_null());
            assert_ne!(dst_actual_size, 0);

            let array = std::slice::from_raw_parts_mut(dst, dst_actual_size as usize);
            let s = str::from_utf8(array).unwrap();
            let json_returned = s.to_string();

            assert_eq!(json_expected.len(), json_returned.len());
            assert_eq!(json_expected, json_returned);
        }
    }
}
