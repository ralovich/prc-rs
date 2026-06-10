// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#[cfg(test)]
pub mod tests {
    use bitstream_io::BitWrite;
    use std::fs::File;
    use std::io::Read;

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

    /// Read whole file into memory.
    pub fn get_file_as_byte_vec(filename: &std::string::String) -> Vec<u8> {
        let mut f = File::open(&filename).expect("no file found");
        let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read_exact(&mut buffer).expect("buffer overflow");

        buffer
    }
}
