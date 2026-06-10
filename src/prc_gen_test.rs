// -*- mode: rust; coding: utf-8-unix -*-
//
// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.
//

#[cfg(test)]
mod tests {
    use crate::builtin;
    use crate::builtin::Boolean;
    use crate::builtin::UnsignedInteger;
    use crate::common::{ParsedPrc, PrcParsingContext};
    use crate::constants::PrcType;
    use crate::prc_gen::*;
    use crate::test_common::tests::*;
    use bitstream_io::{BigEndian, BitReader, BitWriter};
    use std::io::Cursor;

    macro_rules! function {
        () => {{
            fn f() {}
            fn type_name_of<T>(_: T) -> &'static str {
                std::any::type_name::<T>()
            }
            let name = type_name_of(f);
            &name[..name.len() - 3]
        }};
    }

    #[test]
    fn io_globals() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureGlobals = Default::default();
        reference.id.value = PrcType::PRC_TYPE_ASM_FileStructureGlobals as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 12usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_tree() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureTree = Default::default();
        reference.id.value = PrcType::PRC_TYPE_ASM_FileStructureTree as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        reference.internal_data.id.value = PrcType::PRC_TYPE_ASM_FileStructure as u32;
        reference.internal_data.base.entity_name.name = Some(builtin::String {
            value: "dummy2".to_owned(),
        });
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 21usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_tess() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureTessellation = Default::default();
        reference.id.value = PrcType::PRC_TYPE_ASM_FileStructureTessellation as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 11usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered =
            PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_geom() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureGeometry = Default::default();
        reference.id.value = PrcType::PRC_TYPE_ASM_FileStructureGeometry as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 11usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered = PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_extgeom() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureExtraGeometry = Default::default();
        reference.id.value = PrcType::PRC_TYPE_ASM_FileStructureExtraGeometry as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 11usize);

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
        reference.id.value = PrcType::PRC_TYPE_ASM_ModelFile as u32;
        reference.base.entity_name.name = Some(builtin::String {
            value: "dummy1".to_owned(),
        });
        reference.units_from_cad_file.value = true;
        reference.units_in_mm.value = 0.01;
        reference
            .product_occurrences
            .push(ProductOccurrenceReference {
                unique_id: UniqueId {
                    unique_id0: builtin::UnsignedInteger { value: 0 },
                    unique_id1: builtin::UnsignedInteger { value: 1 },
                    unique_id2: builtin::UnsignedInteger { value: 2 },
                    unique_id3: builtin::UnsignedInteger { value: 3 },
                },
                root_index: UnsignedInteger { value: 9 },
                product_occurrence_is_active: Boolean { value: true },
            });
        reference.number_of_root_product_occurrences.value =
            reference.product_occurrences.len() as u32;
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 26usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered = PRC_TYPE_ASM_ModelFile::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_round_trip_prc_json() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[{}] The current directory is {}",
            function!(),
            path.display()
        );
        let bytes_external =
            get_file_as_byte_vec(&std::string::String::from("testdata/yellowtri2.json"));
        //#[cfg(not(target_os = "windows"))]
        //assert_eq!(bytes_external.len(), 28147usize);

        let mut parsed_prc: ParsedPrc = serde_json::from_slice(bytes_external.as_slice()).unwrap();
        assert_eq!(parsed_prc.verread, 7094);
        assert_eq!(parsed_prc.fsi.len(), 1);
        assert_eq!(parsed_prc.uncompr_files.len(), 0);

        parsed_prc.verread = 7095;
        let ser = serde_json::to_string(&parsed_prc).unwrap();
        let bytes = ser.as_bytes();
        //#[cfg(not(target_os = "windows"))]
        //assert_eq!(bytes.len(), 11946usize);
        //#[cfg(not(target_os = "windows"))]
        //assert_eq!(bytes_external, bytes);
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let first_name = json.get("verread").unwrap();
        assert_eq!(first_name.as_i64().unwrap(), 7095);

        let parsed_prc2: ParsedPrc = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed_prc, parsed_prc2);

        // TODO: roundtrip binary .prc
    }

    #[test]
    fn io_byte_based() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[{}] The current directory is {}",
            function!(),
            path.display()
        );

        let test_cases = [
            "testdata/sample-chevrolet-camaro-2014-rs-medium_r10325.stream-137.prc".to_string(),
            //"testdata/A700000010220782.stream-8.prc".to_string(), // needs writing schema contents...
            //"testdata/pmi_sample.stream-23.prc".to_string(), // fails inside ContentCompressedFace due to _ctx.get_surface_type().unwrap());
            //"testdata/3D-PDF-Sample-School.stream-48.prc".to_string(),
        ];

        for test_case in test_cases.iter() {
            println!("\n[{}]Test case: {}", function!(), test_case);

            let bytes = get_file_as_byte_vec(
                &/*"testdata/sample-chevrolet-camaro-2014-rs-medium_r10325.stream-137.prc".to_string(),*/test_case,
            );
            let mut ctx: PrcParsingContext = Default::default();
            {
                let mut rdr = Cursor::new(&bytes);
                let prc = UncompressedFileHeader::from_reader(&mut rdr, &mut ctx).unwrap();
                assert_eq!(b"PRC", prc.magic.a.as_slice());
                /*assert_eq!(8137, prc.minimal_version_for_read.value);
                assert_eq!(8137, prc.authoring_version.value);
                assert_eq!(495, prc.mf_start_offset.value);
                assert_eq!(516, prc.mf_end_offset.value);*/

                prc.decompress_sections(
                    &mut rdr,
                    &mut ctx,
                    bytes.len(),
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                )
                .unwrap();
                //println!("{:#?}", prc);
            }

            // serialize and round-trip
            {
                let mut bytes2: Vec<u8> = vec![];

                let _prc2 =
                    UncompressedFileHeader::compress_and_write(&mut bytes2, &mut ctx).unwrap();
                //prc2.to_writer(&mut bytes2, &mut ctx).unwrap();
                //println!("{:#?}", _prc2);
                assert_eq!(bytes, bytes2);

                let mut rdr = Cursor::new(&bytes);
                let prc3 = UncompressedFileHeader::from_reader(&mut rdr, &mut ctx).unwrap();
                assert_eq!(b"PRC", prc3.magic.a.as_slice());
                /*assert_eq!(8137, prc3.minimal_version_for_read.value);
                assert_eq!(8137, prc3.authoring_version.value);
                assert_eq!(495, prc3.mf_start_offset.value);
                assert_eq!(516, prc3.mf_end_offset.value);*/
            }
        }
    }
}
