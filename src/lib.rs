// -*- mode: rust; coding: utf-8-unix -*-
//
// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.
//

#[cfg(fuzzing)]
pub mod builtin;
#[cfg(not(fuzzing))]
mod builtin;
mod builtin_ana;
mod builtin_byte_aligned;
pub mod capi;
pub mod common;
mod compressed_nurbs;
#[cfg(fuzzing)]
pub mod constants;
#[cfg(not(fuzzing))]
mod constants;
mod decompress;
mod double;
mod huffman;
mod indent;
#[cfg(fuzzing)]
pub mod limits;
#[cfg(not(fuzzing))]
mod limits;
#[cfg(fuzzing)]
pub mod prc_gen;
#[cfg(not(fuzzing))]
mod prc_gen;
mod prc_gen_test;
mod schema;
mod tess_3d_compressed;
mod tess_3d_wire;
#[cfg(fuzzing)]
pub mod test_common;
#[cfg(not(fuzzing))]
mod test_common;
mod vec3;

/// Library/crate version.
#[allow(unused)]
const LIBPRC_VERSION: &str = env!("CARGO_PKG_VERSION");
#[allow(unused)]
const LIBPRC_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
#[allow(unused)]
const LIBPRC_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
#[allow(unused)]
const LIBPRC_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
/// Version of the PRC specification implemented.
const LIBPRC_PRC_SPEC_VERSION: u32 = 8137;
/// Version of the JSON representation.
const LIBPRC_JSON_SCHEMA_VERSION: u32 = 20260910;
