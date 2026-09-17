// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use bitstream_io::BitWrite;

/// fill partial byte at the end
/// returns number of trailing padding bits appended
pub fn fill_partial_byte_at_end<W: BitWrite + ?Sized>(
    w: &mut W,
    bit: bool,
) -> std::io::Result<usize> {
    let mut trailing_bits: usize = 0;
    while !w.byte_aligned() {
        crate::builtin::write_bits(w, bit as u8, 1)?;
        trailing_bits += 1;
    }
    Ok(trailing_bits)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use bitstream_io::{BitWrite, BitWriter};

    #[test]
    fn test_padding() {
        let mut bytes: Vec<u8> = vec![];
        let mut w = BitWriter::endian(&mut bytes, bitstream_io::LittleEndian);
        w.write_bit(false).unwrap();
        let _padding_bits = fill_partial_byte_at_end(&mut w, true).unwrap();
        assert_eq!(7, _padding_bits);
    }
}
