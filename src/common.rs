// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(non_snake_case)]
#![allow(unused)]

use crate::constants::*;
use crate::prc_builtin::*;
use crate::prc_gen::*;
use crate::prc_schema::SchemaEvaluator;
use bitstream_io::BitReader;
use log::{debug, info};
use std::fs::File;
use std::io;
use std::io::{Cursor, Seek};
use std::path::Path;
//use std::rc::Rc;
use measure_time::debug_time;

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

#[derive(Default)]
pub struct PrcParsingContext {
    pub ver_for_reading: u32,
    pub ver_authoring: u32,
    pub current_name: std::string::String,
    pub layer_index: u32,
    pub index_of_line_style: u32,
    pub behavior_bit_field: u16,
    pub se: SchemaEvaluator,

    //pub is_an_iso_face: bool, // ContentCompressedFace

    // iso-vertices (iso-cache) // https://github.com/pdf-association/pdf-issues/issues/705#issuecomment-3697983893
    // ana-vertices (ana-cache)
    pub ContentCurve_is_3d_flag: bool,
    pub PRC_TYPE_CRV_NURBS_is_rational: bool,

    //pub number_of_bits_to_store_reference: u32,
    pub curve_trimming_face: bool, // is TRUE if this compressed line is part of a PRC_TYPE_TOPO_BrepDataCompress; it is FALSE if this compressed line is a part of a PRC_TYPE_TOPO_SingleWireBodyCompress.
    pub compressed_iso_spline: bool, // is TRUE if the circle is being used as the trim boundary of an PRC_HCG_IsoNurbs; otherwise it is FALSE
    //surface_type: u32, // PRC_HCG_...
    current_face_type: Vec<u32>,               // stack of PRC_HCG_...
    pub CompressedNurbs_number_ccpt_in_u: u32, // Number_ccpt_in_u = sumOf(mult_u) - degree_in_u - 1
    //Number_ccpt_in_v = sumOf(mult_v) - degree_in_v - 1
    pub CompressedNurbs_number_ccpt_in_v: u32, // CompressedControlPoints https://github.com/pdf-association/pdf-issues/issues/663

    pub CompressedKnots_number_bit_parameter: u32, // the number of bits used to store knots

    pub BrepDataCompress_sum_num_faces: u32,
    #[allow(non_snake_case)]
    pub BrepDataCompress_number_of_bits_to_store_reference: u32,

    pub brep_data_compressed_tolerance: f64,
    pub nurbs_tolerance: f64, /*= self.brep_data_compressed_tolerance / 5.0*/
    pub CompressedNurbs_number_stored_knots_in_u: u32, /*= self.number_of_knots_in_u ‐ 2*/
    pub CompressedNurbs_number_stored_knots_in_v: u32, /*= self.number_of_knots_in_v – 2*/
    pub CompressedNurbs_number_bits_u: u32, /*= (degree_in_u ? ceil[ log( degree_in_u + 2 ) / log(2) ] : 2)*/
    pub CompressedNurbs_number_bits_v: u32, /*= (degree_in_v ? ceil[ log( degree_in_v + 2 ) / log(2) ] : 2)*/
    pub tolerance_parameter: f64,           /*= 1./ 2^( number_bit_parameter ‐1)*/

    pub CompressedNurbs_number_of_bits_for_isomin: u32, // number of bits used to store first row and column of control points
    pub CompressedNurbs_number_of_bits_for_rest: u32, // number of bits to store the remainder of the control points

    pub VertexColors_number_of_colors: u32,

    // scope is per BrepDataCompress?
    // comes from RefOrCompressedCurve
    // The flag curve_is_not_already_stored indicates if the trim curve has already been stored in the
    // compressed brep data. If the curve has already been stored, the index of the curve is stored in the file;
    // otherwise, a compressed version of the trim curve is stored.
    pub curves: Vec<CompressedCurve>,
}
impl PrcParsingContext {
    pub fn push_face_type(&mut self, id: u32) {
        //println!("Pushing face {}", id);
        self.current_face_type.push(id);
    }
    pub fn pop_face_type(&mut self) {
        // println!(
        //     "Popping face {}",
        //     self.current_face_type[self.current_face_type.len() - 1]
        // );
        self.current_face_type.pop();
    }
    pub fn get_surface_type(&mut self) -> Option<u32> {
        if self.current_face_type.is_empty() {
            return None;
        }
        Some(self.current_face_type[self.current_face_type.len() - 1])
    }
    pub fn on_brep_data_compress(&mut self, _bdc: &PRC_TYPE_TOPO_BrepDataCompress) {
        self.nurbs_tolerance = self.brep_data_compressed_tolerance / 5.0;
        //self.number_stored_knots_in_u = bdc.number_of_knots_in_u ‐ 2;
        panic!("Not implemented!");
    }

    pub fn store_compressed_curve(&mut self, crv: CompressedCurve) {
        self.curves.push(crv);
    }
}

pub struct CurrentFaceType {
    pub value: u32,
    ctx: std::rc::Rc<PrcParsingContext>,
}
impl CurrentFaceType {
    pub fn new(value: u32, mut ctx: std::rc::Rc<PrcParsingContext>) -> CurrentFaceType {
        //let a = std::rc::Rc::downgrade(&ctx);
        std::rc::Rc::get_mut(&mut ctx)
            .unwrap()
            .push_face_type(value);
        //ctx.push_face_type(value);
        CurrentFaceType { value, ctx }
    }
}
impl Drop for CurrentFaceType {
    fn drop(&mut self) {
        std::rc::Rc::get_mut(&mut self.ctx).unwrap().pop_face_type();
    }
}

fn prc_parse_globals(
    bytes: &Vec<u8>,
    ctx: &mut PrcParsingContext,
    verbose: bool,
    parse_globals: bool,
) {
    debug!(
        "--prc_parse_globals {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);
    //let mut reader = BitReader::endian(Cursor::new(&data), BigEndian);

    let schema_data = Schema::from_reader(&mut reader, ctx).unwrap();
    if verbose {
        let _schema_str = format!("{:#?}", schema_data);
        debug!("{}", _schema_str);
    }

    ctx.se = SchemaEvaluator::new(&schema_data.schemas);

    if parse_globals {
        let data = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut reader, ctx).unwrap();
        if verbose {
            let _str = format!("{:#?}", data);
            debug!("{}", _str);
        }
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--glob ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );

    ()
}

fn prc_parse_tree(bytes: &Vec<u8>, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_tree {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    let data = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut reader, ctx).unwrap();
    // let mut data = Default::default();
    // data = match PRC_PRC_TYPE_ASM_FileStructureTree::from_reader(&mut reader, ctx) {
    //     Ok(x) => x,
    //     Err(x) => {

    //     },
    // }
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--tree ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_parse_tess(bytes: &Vec<u8>, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_tess {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    let data = PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut reader, ctx).unwrap();
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--tess ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_parse_geom(bytes: &Vec<u8>, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_geom {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    let data = PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut reader, ctx).unwrap();
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--geom ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_parse_extgeom(bytes: &Vec<u8>, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_extgeom {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    let data = PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut reader, ctx).unwrap();
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--extgeom ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_parse_modfile(bytes: &Vec<u8>, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_modfile {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    let schema_data = Schema::from_reader(&mut reader, ctx);
    let _schema_str = format!("{:#?}", schema_data);
    if verbose {
        //debug!("{}", _schema_str);
    }

    let data = PRC_TYPE_ASM_ModelFile::from_reader(&mut reader, ctx);
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--modfile ENDOK remaining: {} bits, consumed bits: {} of {} ({} bytes)  [took {} ms]--",
        remaining_bits,
        consumed_bits,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_dump(fname: &std::string::String, data: &Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(fname);
    //let file = File::create(path.as_ref())?;
    //let mut writer = std::io::BufWriter::new(file);
    //file.write(_data)?;
    std::fs::write(path, data)?;

    Ok(())
}

// TODO: this function should not panic, but return Err instead
pub fn prc_describe(
    fname: &std::string::String,
    verbose: bool,
    all: bool,
    globals: bool,
    tree: bool,
    tess: bool,
    geom: bool,
    extgeom: bool,
    _schema: bool,
) -> io::Result<()> {
    debug_time!("prc_describe");

    // Create a path to the desired file
    let path = Path::new(fname);
    let display = path.display();

    info!("--parsing \"{}\"--", display);

    let mut now = std::time::Instant::now();
    // Open the path in read-only mode, returns `io::Result<File>`
    let mut _file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    let bytes: Vec<u8> = std::fs::read(fname)?;
    debug!("read {} bytes", bytes.len());

    let file_size_bytes = bytes.len();
    let mut mem_reader: Cursor<Vec<u8>> = Cursor::new(bytes);
    debug!(
        "Reading into memory [took {} ms]",
        now.elapsed().as_millis()
    );

    let config = PRCHeader::from_reader(&mut mem_reader, file_size_bytes);
    let a = &config?;

    let mut ctx: PrcParsingContext = Default::default();
    ctx.ver_for_reading = a.verread;
    ctx.ver_authoring = a.verauth;

    // parse uncompressed files
    // TODO: could be processed concurrently
    // TODO: is it possible to use multiple,cloned contexts and merge them later?
    for i in 0..a.fsi.len() {
        debug_time!("--fsi={}--", i);
        prc_parse_globals(
            &a.fsi[i].sections[PRCSectionKind::Global as usize],
            &mut ctx,
            verbose,
            all || globals,
        );
        if all || tree {
            prc_parse_tree(
                &a.fsi[i].sections[PRCSectionKind::Tree as usize],
                &mut ctx,
                verbose,
            );
        }
        if all || tess {
            prc_parse_tess(
                &a.fsi[i].sections[PRCSectionKind::Tessellation as usize],
                &mut ctx,
                verbose,
            );
        }
        if all || geom {
            prc_parse_geom(
                &a.fsi[i].sections[PRCSectionKind::Geometry as usize],
                &mut ctx,
                verbose,
            );
        }
        if all || extgeom {
            prc_parse_extgeom(
                &a.fsi[i].sections[PRCSectionKind::ExtraGeometry as usize],
                &mut ctx,
                verbose,
            );
        }
    }
    if all || globals || tree || tess || geom || extgeom {
        // parse model file
        prc_parse_modfile(&a.mf, &mut ctx, verbose);
    }

    info!("--parsed successfully \"{}\"--", display);
    Ok(())
}

pub fn prc_explode(fname: &std::string::String) -> io::Result<()> {
    let path = Path::new(fname);
    let display = path.display();

    println!("--parsing \"{}\"--", display);

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut _file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    let bytes = std::fs::read(fname).unwrap();

    let file_size_bytes = bytes.len();
    let mut mem_reader: Cursor<Vec<u8>> = Cursor::new(bytes);

    // Result<PRCHeader, std::io::Error>
    let config = PRCHeader::from_reader(&mut mem_reader, file_size_bytes);
    let a = &config?;

    let base_name = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();
    for i in 0..a.fsi.len() {
        let base = base_name.replace(".prc", format!("_{i}_glob.bin").as_str());
        let _ = prc_dump(&base, &a.fsi[i].sections[PRCSectionKind::Global as usize]);
        let base = base_name.replace(".prc", format!("_{i}_tree.bin").as_str());
        let _ = prc_dump(&base, &a.fsi[i].sections[PRCSectionKind::Tree as usize]);
        let base = base_name.replace(".prc", format!("_{i}_tess.bin").as_str());
        let _ = prc_dump(
            &base,
            &a.fsi[i].sections[PRCSectionKind::Tessellation as usize],
        );
        let base = base_name.replace(".prc", format!("_{i}_geom.bin").as_str());
        let _ = prc_dump(&base, &a.fsi[i].sections[PRCSectionKind::Geometry as usize]);
        let base = base_name.replace(".prc", format!("_{i}_extg.bin").as_str());
        let _ = prc_dump(
            &base,
            &a.fsi[i].sections[PRCSectionKind::ExtraGeometry as usize],
        );
    }
    let base = base_name.replace(".prc", format!("_mf.bin").as_str());
    let _ = prc_dump(&base, &a.mf);
    for i in 0..a.uncompr_files.len() {
        let base = base_name.replace(".prc", format!("_ucmp_{i}.bin").as_str());
        let _ = prc_dump(&base, &a.uncompr_files[i]);
    }

    Ok(())
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
