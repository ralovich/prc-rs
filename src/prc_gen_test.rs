// -*- mode: rust; coding: utf-8-unix -*-
//
// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.
//

#[cfg(test)]
mod tests {
    use crate::builtin;
    use crate::builtin::{Boolean, Character, Double, Integer, UnsignedInteger};
    use crate::common::{ParsedPrc, PrcParsingContext};
    use crate::constants::{PrcType, TextureMappingType};
    use crate::prc_gen::*;
    use crate::test_common::*;
    use bitstream_io::{BitReader, BitWriter};
    use std::io::Cursor;

    const ENDIAN: bitstream_io::BigEndian = bitstream_io::BigEndian;

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
        let reference: PRC_TYPE_ASM_FileStructureGlobals = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureGlobals as u32
        );
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 5usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_globals_all() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_FileStructureGlobals = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureGlobals as u32
        );
        reference.file_count.value = 1;
        reference.unique_ids.push(Default::default());
        reference.global_data.serialize_help.font_keys_count.value = 1;
        reference
            .global_data
            .serialize_help
            .font_keys_of_font
            .push(Default::default());
        reference.global_data.serialize_help.font_keys_of_font[0]
            .key_count
            .value = 1;
        reference.global_data.serialize_help.font_keys_of_font[0]
            .font_key_list
            .push(Default::default());
        reference.global_data.color_count.value = 1;
        reference.global_data.colors.push(Default::default());
        reference.global_data.picture_count.value = 1;
        reference.global_data.pictures.push(Default::default());
        reference.global_data.texture_count.value = 1;
        reference.global_data.textures.push(Default::default());
        reference.global_data.textures[0].texture_dimension.value = 2;
        reference.global_data.textures[0].texture_mapping_type.value =
            TextureMappingType::Operator as i32;
        reference.global_data.textures[0].texture_mapping_operator = Some(Integer { value: 1 });
        reference.global_data.textures[0].has_transformation = Some(Boolean { value: true });
        reference.global_data.textures[0].transformation = Some(Default::default());
        reference.global_data.textures[0]
            .number_of_texture_mapping_attributes_intensities
            .value = 1;
        reference.global_data.textures[0].texture_mapping_attributes_intensities =
            Some(vec![Double { value: 1.0 }]);
        reference.global_data.textures[0]
            .number_of_texture_mapping_attributes_components
            .value = 1;
        reference.global_data.textures[0].texture_mapping_attributes_components =
            Some(vec![Character { value: 127 }]);
        reference.global_data.textures[0].texture_wrapping_mode_t = Some(Integer { value: 1 });
        reference.global_data.textures[0]
            .has_texture_transformation
            .value = true;
        reference.global_data.textures[0].texture_transformation = Some(Default::default());
        reference.global_data.textures[0]
            .texture_transformation
            .as_mut()
            .unwrap()
            .transform_2d
            .value = true;
        reference.global_data.textures[0]
            .texture_transformation
            .as_mut()
            .unwrap()
            .transform = Some(Default::default());
        reference.global_data.material_count.value = 2;
        reference.global_data.materials.push(Default::default());
        reference.global_data.materials[0].id_concrete = Material_idConcrete::m(Default::default());
        reference.global_data.materials.push(Default::default());
        reference.global_data.materials[1].id_concrete =
            Material_idConcrete::ta(Default::default());
        reference.global_data.line_pattern_count.value = 1;
        reference.global_data.line_patterns.push(Default::default());
        reference.global_data.line_patterns[0]
            .number_of_elements
            .value = 1;
        reference.global_data.line_patterns[0]
            .length
            .push(Default::default());
        reference.global_data.style_count.value = 1;
        reference.global_data.styles.push(Default::default());
        reference.global_data.fill_count.value = 4;
        reference.global_data.fills.push(Default::default());
        reference.global_data.fills[0].data_concrete =
            PRC_TYPE_GRAPH_FillPattern_dataConcrete::dp(Default::default());
        reference.global_data.fills.push(Default::default());
        reference.global_data.fills[1].data_concrete =
            PRC_TYPE_GRAPH_FillPattern_dataConcrete::hp(Default::default());
        if let PRC_TYPE_GRAPH_FillPattern_dataConcrete::hp(hp) =
            &mut reference.global_data.fills[1].data_concrete
        {
            hp.number_of_hatching_lines.value = 1;
            hp.hatch.push(Default::default());
        }
        reference.global_data.fills.push(Default::default());
        reference.global_data.fills[2].data_concrete =
            PRC_TYPE_GRAPH_FillPattern_dataConcrete::sp(Default::default());
        reference.global_data.fills.push(Default::default());
        reference.global_data.fills[3].data_concrete =
            PRC_TYPE_GRAPH_FillPattern_dataConcrete::vpp(Default::default());
        if let PRC_TYPE_GRAPH_FillPattern_dataConcrete::vpp(vpp) =
            &mut reference.global_data.fills[3].data_concrete
        {
            vpp.markup
                .tessellation_coordinates
                .number_of_coordinates
                .value = 3;
            vpp.markup
                .tessellation_coordinates
                .coordinates
                .push(Double { value: 1.0 });
            vpp.markup
                .tessellation_coordinates
                .coordinates
                .push(Double { value: 2.0 });
            vpp.markup
                .tessellation_coordinates
                .coordinates
                .push(Double { value: 3.0 });
            vpp.markup.number_of_codes.value = 1;
            vpp.markup.code_numbers.push(UnsignedInteger { value: 0 });
            vpp.markup.number_of_text_strings.value = 1;
            vpp.markup.text_strings.push(crate::builtin::String {
                value: "vpp markup".to_owned(),
            });
            vpp.markup.tessellation_label.value = "label".to_string();
        }
        reference.global_data.ref_coord_count.value = 2;
        reference.global_data.ref_coords.push(Default::default());
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .entity_name
            .same_name
            .value = false;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .entity_name
            .name = Some(builtin::String::from("bla"));
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attribute_count
            .value = 1;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes
            .push(Default::default());
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attribute_title
            .flag
            .value = false;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attribute_title
            .string_title = Some(builtin::String {
            value: "string".to_owned(),
        });
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .number_of_attributes
            .value = 1;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes
            .push(Default::default());
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes[0]
            .title
            .flag
            .value = true;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes[0]
            .title
            .integer_title = Some(UnsignedInteger::from(500));
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes[0]
            .title
            .string_title = None;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes[0]
            .type_
            .value = 2;
        reference.global_data.ref_coords[0]
            .item_content
            .base
            .base
            .base
            .attribute_data
            .attributes[0]
            .attributes[0]
            .valued = Some(Double::from(6.78));
        reference.global_data.ref_coords[0].transform_concrete =
            crate::prc_gen::PRC_TYPE_RI_CoordinateSystem_transformConcrete::ct(Default::default());
        reference.global_data.ref_coords[0]
            .user_data
            .data
            .push(true);
        reference.global_data.ref_coords.push(Default::default());
        reference.global_data.ref_coords[1].transform_concrete =
            crate::prc_gen::PRC_TYPE_RI_CoordinateSystem_transformConcrete::gt(Default::default());
        reference.global_data.ref_coords[1]
            .user_data
            .data
            .push(true);
        reference.user_data.data.push(true);
        println!("{:#?}", reference);
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 155usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_tree() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let reference: PRC_TYPE_ASM_FileStructureTree = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureTree as u32
        );
        assert_eq!(
            reference.internal_data.id.value,
            PrcType::PRC_TYPE_ASM_FileStructure as u32
        );
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 7usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_tess() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let reference: PRC_TYPE_ASM_FileStructureTessellation = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureTessellation as u32
        );
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 3usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered =
            PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_geom() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let reference: PRC_TYPE_ASM_FileStructureGeometry = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureGeometry as u32
        );
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 3usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered = PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_extgeom() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let reference: PRC_TYPE_ASM_FileStructureExtraGeometry = Default::default();
        assert_eq!(
            reference.id.value,
            PrcType::PRC_TYPE_ASM_FileStructureExtraGeometry as u32
        );
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 3usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        let recovered =
            PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(reference, recovered);
    }

    #[test]
    fn io_mf() {
        let mut ctx: PrcParsingContext = Default::default();
        let mut bytes: Vec<u8> = vec![];
        let mut reference: PRC_TYPE_ASM_ModelFile = Default::default();
        assert_eq!(reference.id.value, PrcType::PRC_TYPE_ASM_ModelFile as u32);
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
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
            let _ = reference.to_writer(&mut w, &mut ctx);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 18usize);

        let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
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
            std::fs::read(std::string::String::from("testdata/yellowtri2.json")).unwrap();
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
        let json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let first_name = json.get("verread").unwrap();
        assert_eq!(first_name.as_i64().unwrap(), 7095);

        let parsed_prc2: ParsedPrc = serde_json::from_slice(bytes).unwrap();
        assert_eq!(parsed_prc, parsed_prc2);

        {
            let bytes_external3 = std::fs::read(std::string::String::from(
                "testdata/yellowtri2_with_uncompr_files.json",
            ))
            .unwrap();
            let parsed_prc3: ParsedPrc =
                serde_json::from_slice(bytes_external3.as_slice()).unwrap();

            let mut parsed_prc4 = parsed_prc2.clone();
            parsed_prc4.verread = 7094;
            parsed_prc4.uncompr_files.push(vec![1, 2, 3, 4, 255]);
            parsed_prc4.uncompr_files.push(vec![255, 128, 0, 1]);

            assert_eq!(parsed_prc3, parsed_prc4);
        }

        // TODO: roundtrip binary .prc
    }
    #[test]
    fn io_valid_default() {
        let n = crate::prc_gen::Name::default();
        assert_eq!(n.same_name.value, false);
        assert!(n.name.is_some());
    }
    #[test]
    fn io_generated_ctor_works() {
        let attr = crate::prc_gen::PRC_TYPE_MISC_Attribute::default();
        assert_eq!(
            attr.id.value,
            crate::constants::PrcType::PRC_TYPE_MISC_Attribute as u32
        );
    }

    #[test]
    fn io_all_prc_structs() {}
}
