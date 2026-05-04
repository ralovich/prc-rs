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
    use std::fs::File;
    //use super::*;
    use crate::common::{ParsedPrc, PrcParsingContext};
    use crate::prc_builtin;
    use crate::prc_builtin::UnsignedInteger;
    use crate::prc_gen::*;
    use bitstream_io::{BigEndian, BitReader, BitWrite, BitWriter};
    use std::io::{Cursor, Read};

    /// fill partial byte at the end
    fn fill_partial_byte_at_end<W: BitWrite + ?Sized>(w: &mut W) -> std::io::Result<()> {
        while !w.byte_aligned() {
            w.write_bit(false)?;
        }
        Ok(())
    }

    /// Read whole file into memory.
    fn get_file_as_byte_vec(filename: &std::string::String) -> Vec<u8> {
        let mut f = File::open(&filename).expect("no file found");
        let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read_exact(&mut buffer).expect("buffer overflow");

        buffer
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

    #[test]
    fn io_round_trip_prc_json() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[io_round_trip_prc_json] The current directory is {}",
            path.display()
        );
        let bytes_external =
            get_file_as_byte_vec(&std::string::String::from("testdata/yellowtri2.json"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(bytes_external.len(), 11911usize);

        let mut parsed_prc: ParsedPrc = serde_json::from_slice(bytes_external.as_slice()).unwrap();
        assert_eq!(parsed_prc.verread, 7094);
        assert_eq!(parsed_prc.fsi.len(), 1);
        assert_eq!(parsed_prc.uncompr_files.len(), 0);

        parsed_prc.verread = 7095;
        let ser = serde_json::to_string(&parsed_prc).unwrap();
        let bytes = ser.as_bytes();
        #[cfg(not(target_os = "windows"))]
        assert_eq!(bytes.len(), 11909usize);
        //#[cfg(not(target_os = "windows"))]
        //assert_eq!(bytes_external, bytes);
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let first_name = json.get("verread").unwrap();
        assert_eq!(first_name.as_i64().unwrap(), 7095);

        let mut parsed_prc2: ParsedPrc = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed_prc, parsed_prc2);

        // TODO: roundtrip binary .prc
    }
}
