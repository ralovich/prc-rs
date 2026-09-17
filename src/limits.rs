// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

pub const MAX_NUM_STR_LEN: u32 = 4096;
pub const MAX_NUM_USERDATA_BITS: u32 = 8 * 1024 * 256;
pub const MAX_NUM_ARRAY_ELEMENTS: u32 = 1024 * 1024 * 8;

#[macro_export]
macro_rules! io_check_limit {
    ($value:expr, $limit:expr) => {
        if $value > $limit {
            return Err(std::io::Error::other(format!(
                "Violated limit {} ({}) > {} ({})!",
                stringify!($value),
                $value,
                stringify!($limit),
                $limit
            )));
        }
    };
}
