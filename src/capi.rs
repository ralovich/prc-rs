// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use crate::common::prc_describe;

pub type AllocateFnT = ::std::option::Option<unsafe extern "C" fn(argument: usize) -> *mut u8>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prc_parse_to_json(
    src_len: u64,
    src: *const u8,
    allocate: AllocateFnT,
    dst: *mut *mut u8,
    dst_actual_size: *mut u64,
) -> i32 {
    if src_len < 1 {
        return -1;
    }
    if src == std::ptr::null() {
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

    let verbose: bool = true;
    let all: bool = true;
    let globals: bool = true;
    let tree: bool = true;
    let tess: bool = true;
    let geom: bool = true;
    let extgeom: bool = true;
    let _schema: bool = true;
    let modelfile = true;
    let rv = prc_describe(
        src_slice,
        &"capi.prc_parse_to_json.prc".to_owned(),
        verbose,
        all,
        globals,
        tree,
        tess,
        geom,
        extgeom,
        _schema,
        modelfile,
    );
    return match rv {
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
                    *dst = allocate_func(parsed.len());
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
    };
}
