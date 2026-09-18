#![no_main]
#![allow(unused)]

use bitstream_io::{BitReader, BitWriter};
use libfuzzer_sys::{/*fuzz_mutator,*/ fuzz_target};
use prc::prc_gen::{
    PRC_TYPE_ASM_FileStructureExtraGeometry, PRC_TYPE_ASM_FileStructureGeometry,
    PRC_TYPE_ASM_FileStructureGlobals, PRC_TYPE_ASM_FileStructureTessellation,
    PRC_TYPE_ASM_FileStructureTree, PRC_TYPE_ASM_ModelFile, Schema,
};
use std::io::Cursor;

extern crate prc;

const ENDIAN: bitstream_io::BigEndian = bitstream_io::BigEndian;

fuzz_target!(|data: &[u8]| {
    let mut ctx = prc::common::PrcParsingContext::default();

    let mut r = BitReader::endian(Cursor::new(&data), ENDIAN);
    if data.len() < 1 {
        return;
    }
    if data[0] == 0 {
        let _a = Schema::from_reader(&mut r, &mut ctx);
    } else if data[0] == 1 {
        let _a = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx);
    } else if data[0] == 2 {
        let _a = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut r, &mut ctx);
    } else if data[0] == 3 {
        let _a = PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut r, &mut ctx);
    } else if data[0] == 4 {
        let _a = PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut r, &mut ctx);
    } else if data[0] == 5 {
        let _a = PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut r, &mut ctx);
    } else if data[0] == 6 {
        let _a = PRC_TYPE_ASM_ModelFile::from_reader(&mut r, &mut ctx);
    } else if data[0] == 7 {
        let _a = prc::builtin::Double::from_reader(&mut r);
    }
});
