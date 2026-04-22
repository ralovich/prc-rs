// -*- mode: rust; coding: utf-8-unix -*-
//
// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.
//

#![allow(unreachable_code)]
#![allow(unused)]

#[cfg(test)]
mod tests {
    //use super::*;
    use crate::common::PrcParsingContext;
    use crate::prc_builtin;
    use crate::prc_builtin::UnsignedInteger;
    use crate::prc_gen::*;
    use bitstream_io::{BigEndian, BitReader, BitWrite, BitWriter};
    use std::io::Cursor;

    /// fill partial byte at the end
    fn fill_partial_byte_at_end<W: BitWrite + ?Sized>(w: &mut W) -> std::io::Result<()> {
        while !w.byte_aligned() {
            w.write_bit(false)?;
        }
        Ok(())
    }
    /*
        #[test]
        fn io_globals() {
            let mut ctx: PrcParsingContext = Default::default();
            let mut bytes: Vec<u8> = vec![];
            let mut reference: PRC_TYPE_ASM_FileStructureGlobals = Default::default();
            reference.id = UnsignedInteger {
                value: prc_builtin::PRCType::PRC_TYPE_ASM_FileStructureGlobals as u32,
            };
            {
                let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
                let _ = reference.to_writer(&mut w, &mut ctx);
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
            }
            assert_eq!(bytes.len(), 5usize);

            let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
            let recovered =
                PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx).unwrap();
            assert_eq!(reference, recovered);
        }

        #[test]
        fn io_tree() {
            let mut ctx: PrcParsingContext = Default::default();
            let mut bytes: Vec<u8> = vec![];
            let mut reference: PRC_TYPE_ASM_FileStructureTree = Default::default();
            reference.id.value = prc_builtin::PRCType::PRC_TYPE_ASM_FileStructureTree as u32;
            reference.internal_data.id.value = prc_builtin::PRCType::PRC_TYPE_ASM_FileStructure as u32;
            {
                let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
                let _ = reference.to_writer(&mut w, &mut ctx);
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
            }
            assert_eq!(bytes.len(), 7usize);

            let mut ctx: PrcParsingContext = Default::default();
            let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
            let recovered = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut r, &mut ctx).unwrap();
            assert_eq!(reference, recovered);
        }

        #[test]
        fn io_extgeom() {
            let mut ctx: PrcParsingContext = Default::default();
            let mut bytes: Vec<u8> = vec![];
            let mut reference: PRC_TYPE_ASM_FileStructureExtraGeometry = Default::default();
            reference.id = UnsignedInteger {
                value: prc_builtin::PRCType::PRC_TYPE_ASM_FileStructureExtraGeometry as u32,
            };
            {
                let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
                let _ = reference.to_writer(&mut w, &mut ctx);
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
            }
            assert_eq!(bytes.len(), 3usize);

            let mut ctx: PrcParsingContext = Default::default();
            let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
            let recovered =
                PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut r, &mut ctx).unwrap();
            assert_eq!(reference, recovered);
        }

        #[test]
        fn io_mf() {
            let mut ctx: PrcParsingContext = Default::default();
            let mut bytes: Vec<u8> = vec![];
            let mut reference: PRC_TYPE_ASM_ModelFile = Default::default();
            reference.id = UnsignedInteger {
                value: prc_builtin::PRCType::PRC_TYPE_ASM_ModelFile as u32,
            };
            {
                let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
                let _ = reference.to_writer(&mut w, &mut ctx);
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
            }
            assert_eq!(bytes.len(), 4usize);

            let mut ctx: PrcParsingContext = Default::default();
            let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
            let recovered = PRC_TYPE_ASM_ModelFile::from_reader(&mut r, &mut ctx).unwrap();
            assert_eq!(reference, recovered);
        }
    */
}
