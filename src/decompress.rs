// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use crate::function;
use deflate::deflate_bytes_zlib;
use inflate::inflate_bytes_zlib;
use measure_time::debug_time;
use std::io;

pub fn decompress(section_compressed: &[u8]) -> std::io::Result<Vec<u8>> {
    debug_time!("{}", function!());
    match inflate_bytes_zlib(section_compressed) {
        Ok(decomp) => Ok(decomp),
        Err(e) => Err(io::Error::other(e)),
    }
}

pub fn compress(section_uncompressed: &[u8]) -> Result<Vec<u8>, String> {
    debug_time!("{}", function!());
    Ok(deflate_bytes_zlib(section_uncompressed))
}
