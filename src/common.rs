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
use crate::tess_3d_compressed::Tess3dCompressed;
use bitstream_io::BitReader;
use log::{debug, info, warn};
use std::fs::File;
use std::io;
use std::io::{Cursor, Seek};
use std::path::Path;
//use std::rc::Rc;
use measure_time::debug_time;
use serde::{Deserialize, Serialize};
//use bson::{bson, Bson};

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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ParsedPrcFileStructure {
    pub uuid0: u32,
    pub uuid1: u32,
    pub uuid2: u32,
    pub uuid3: u32,
    pub schema: Schema,
    pub globals: PRC_TYPE_ASM_FileStructureGlobals,
    pub tree: PRC_TYPE_ASM_FileStructureTree,
    pub tess: PRC_TYPE_ASM_FileStructureTessellation,
    pub geom: PRC_TYPE_ASM_FileStructureGeometry,
    pub ext: PRC_TYPE_ASM_FileStructureExtraGeometry,
}

/// All information from a parsed PRC. Can be (de-)serialized into e.g. JSON.
#[derive(Serialize, Deserialize, Default)]
pub struct ParsedPrc {
    pub verread: u32,
    pub verauth: u32,
    pub uuid0: u32,
    pub uuid1: u32,
    pub uuid2: u32,
    pub uuid3: u32,
    pub uuida0: u32,
    pub uuida1: u32,
    pub uuida2: u32,
    pub uuida3: u32,
    pub fsi: Vec<ParsedPrcFileStructure>,
    pub mf_schema: Schema,
    pub mf: PRC_TYPE_ASM_ModelFile,
    pub uncompr_files: Vec<Vec<u8>>,
}
impl ParsedPrc {
    /*
    pub fn encode_into(&self) -> Vec<u8> {
        debug_time!("ParsedPrc::encode_into()");
        let mut strings: Vec<std::string::String> = Vec::with_capacity(self.fsi.len()*6+2);
        for i in 0..self.fsi.len() {
            let serialized = serde_json::to_string(&self.fsi[i].schema).unwrap();
            strings.push(serialized);
            let serialized = serde_json::to_string(&self.fsi[i].globals).unwrap();
            strings.push(serialized);
            let serialized = serde_json::to_string(&self.fsi[i].tree).unwrap();
            strings.push(serialized);
            let serialized = serde_json::to_string(&self.fsi[i].tess).unwrap();
            strings.push(serialized);
            let serialized = serde_json::to_string(&self.fsi[i].geom).unwrap();
            strings.push(serialized);
            let serialized = serde_json::to_string(&self.fsi[i].ext).unwrap();
            strings.push(serialized);
        }
        let serialized = serde_json::to_string(&self.mf_schema).unwrap();
        strings.push(serialized);
        let serialized = serde_json::to_string(&self.mf).unwrap();
        strings.push(serialized);

        if false {
            let mut bsons: Vec<Bson> = Vec::with_capacity(self.fsi.len()*6+2);
            for i in 0..self.fsi.len() {
                bsons.push(bson::serialize_to_bson(&self.fsi[i].schema).unwrap());
                bsons.push(bson::serialize_to_bson(&self.fsi[i].globals).unwrap());
                bsons.push(bson::serialize_to_bson(&self.fsi[i].tree).unwrap());
                bsons.push(bson::serialize_to_bson(&self.fsi[i].tess).unwrap());
                bsons.push(bson::serialize_to_bson(&self.fsi[i].geom).unwrap());
                bsons.push(bson::serialize_to_bson(&self.fsi[i].ext).unwrap());
            }
            bsons.push(bson::serialize_to_bson(&self.mf_schema).unwrap());
            bsons.push(bson::serialize_to_bson(&self.mf).unwrap());
            //for i in 0..bsons.len() {
            //    debug!("bsons[{}] = {:?}", i, bsons[i]));
            //}
        }
        if false {
            let mut num_bytes = 0usize;
            let mut dst = Vec::with_capacity(self.fsi.len()*6+2);
            for i in 0..self.fsi.len() {
                dst.push(bson::serialize_to_vec(&self.fsi[i].schema).unwrap());
                num_bytes += dst.last().unwrap().len();
                dst.push(bson::serialize_to_vec(&self.fsi[i].globals).unwrap());
                num_bytes += dst.last().unwrap().len();
                dst.push(bson::serialize_to_vec(&self.fsi[i].tree).unwrap());
                num_bytes += dst.last().unwrap().len();
                dst.push(bson::serialize_to_vec(&self.fsi[i].tess).unwrap());
                num_bytes += dst.last().unwrap().len();
                dst.push(bson::serialize_to_vec(&self.fsi[i].geom).unwrap());
                num_bytes += dst.last().unwrap().len();
                dst.push(bson::serialize_to_vec(&self.fsi[i].ext).unwrap());
                num_bytes += dst.last().unwrap().len();
            }
            dst.push(bson::serialize_to_vec(&self.mf_schema).unwrap());
            num_bytes += dst.last().unwrap().len();
            dst.push(bson::serialize_to_vec(&self.mf).unwrap());
            num_bytes += dst.last().unwrap().len();
        }

        let mut dst = Vec::new();
        dst.push(b'[');
        for s in strings {
            dst.append(s.as_bytes().to_vec().as_mut());
            dst.push(b',');
            dst.push(b'\n');
        }
        dst.push(b'{');
        dst.push(b'}');
        dst.push(b'\n');
        dst.push(b']');
        dst
    }
     */
}

#[derive(Default)]
pub struct PrcParsingContext {
    pub ver_for_reading: u32,
    pub ver_authoring: u32,

    /// https://github.com/pdf-association/pdf-issues/issues/724
    pub filestructure_count: u32,

    pub current_name: std::string::String,
    pub layer_index: u32,
    pub index_of_line_style: u32,
    pub behavior_bit_field: u16,
    pub se: SchemaEvaluator,

    //pub is_an_iso_face: bool, // ContentCompressedFace
    /// number_vertex_references  is just a portion of the referenced vertices. Technically,
    /// it is a number of referenced iso-vertices, i.e. vertices referenced from iso faces.
    /// In simple words, imagine there are two caches for compressed vertices - one for
    /// iso-vertices (iso-cache), and one for ana-vertices (ana-cache). When a compressed vertex
    /// is referenced by point_index, for iso-vertex get vertex from iso-cache by point_index,
    /// and for ana-vertex get vertex from ana-cache using (point_index - number_vertex_references)
    /// as an index.
    // BRepCompressed.number_vertex_references
    // BRepCompressed.number_edge_references

    // iso-vertices (iso-cache) // https://github.com/pdf-association/pdf-issues/issues/705#issuecomment-3697983893
    // ana-vertices (ana-cache)
    pub ContentCurve_is_3d_flag: bool,
    pub PRC_TYPE_CRV_NURBS_is_rational: bool,

    //pub number_of_bits_to_store_reference: u32,
    /// ISO 2014: is TRUE if this compressed line is part of a PRC_TYPE_TOPO_BrepDataCompress; it is FALSE if this compressed line is a part of a PRC_TYPE_TOPO_SingleWireBodyCompress.
    /// Acrobat SDK 9: group___tf_x_k_circle_____serialize.html
    /// curve_trimming_face indicates that this function is called by the Serialize ContentCompressedFace function.
    ///
    pub curve_trimming_face: bool,

    /// ISO 2014: is TRUE if the circle is being used as the trim boundary of an PRC_HCG_IsoNurbs; otherwise it is FALSE
    /// Acrobat SDK 9: group___tf_x_k_circle_____serialize.html
    /// compressed_iso_spline_serialization is true if this function is called from SerializeCompressedIsoNurbs.
    pub compressed_iso_spline: bool,
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

    /// 8.9.21.10 CompressedVertex
    /// Each compressed brep data serialization maintains an array of previously written
    /// vertices, starting at index 0.
    pub BrepDataCompress_CompressedVertex_array: Vec<Vec<[f64; 3]>>,

    // TODO: scope is per BrepDataCompress?
    // comes from RefOrCompressedCurve
    // The flag curve_is_not_already_stored indicates if the trim curve has already been stored in the
    // compressed brep data. If the curve has already been stored, the index of the curve is stored in the file;
    // otherwise, a compressed version of the trim curve is stored.
    pub AnaFaceTrimLoop_curves: Vec<CompressedCurve>,
    /// the current loop
    pub AnaFaceTrimLoop_loops: Vec<Vec<CompressedCurve>>,
    /// accumulator of loops

    /// Whether the parser is progressing within a PRC_TYPE_TESS_3D_Wire struct?
    TESS_3D_Wire_inside: bool,
    pub t3dc: Tess3dCompressed,

    pub prc_parsed: ParsedPrc,
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

    pub fn BrepDataCompress_enter(&mut self) {
        self.BrepDataCompress_CompressedVertex_array
            .push(Vec::new());
    }
    pub fn BrepDataCompress_leave(&mut self) {
        assert!(!self.BrepDataCompress_CompressedVertex_array.is_empty());
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        debug!(
            "BrepDataCompress_leave: {} vertices",
            self.BrepDataCompress_CompressedVertex_array[last_idx].len()
        );
    }
    pub fn BrepDataCompress_CompressedVertex_add(&mut self, pt: [f64; 3]) {
        if self.BrepDataCompress_CompressedVertex_array.is_empty() {
            warn!("BrepDataCompress_CompressedVertex_array is empty!");
            return;
        }
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        self.BrepDataCompress_CompressedVertex_array[last_idx].push(pt);
    }
    pub fn BrepDataCompress_CompressedVertex_get(&mut self, point_index: u32) {
        if self.BrepDataCompress_CompressedVertex_array.is_empty() {
            warn!("BrepDataCompress_CompressedVertex_array is empty!");
            return;
        }
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        if point_index as usize >= self.BrepDataCompress_CompressedVertex_array[last_idx].len() {
            warn!("BrepDataCompress_CompressedVertex_array is too small!");
            return;
        }
        let pt = self.BrepDataCompress_CompressedVertex_array[last_idx][point_index as usize];
        debug!("ref vtx: {} -> {:?}", point_index, pt);
    }

    pub fn set_curve_trimming_face(&mut self, on: bool) {
        let prev = self.curve_trimming_face;
        self.curve_trimming_face = on;
        warn!(
            "SetCurveTrimmingFace {} -> {}",
            prev, self.curve_trimming_face
        )
    }
    pub fn is_curve_trimming_face(&self) -> bool {
        self.curve_trimming_face
    }

    pub fn set_compressed_iso_spline(&mut self, on: bool) {
        let prev = self.compressed_iso_spline;
        self.compressed_iso_spline = on;
        warn!(
            "set_compressed_iso_spline {} -> {}",
            prev, self.compressed_iso_spline
        );
    }
    pub fn is_compressed_iso_spline(&self) -> bool {
        self.compressed_iso_spline
    }

    /// group___tf3_d_wire_tess_data_____serialize_content2.html
    /// Note that the number of colors is deduced from the number of point indices as calculated from wire_indexes * 3 or 4 (RGB or RGBA).
    /// It is important to remember that implicit points must also have a color (see Special flags for 3DWireTessData tessellation).
    pub fn set_num_vertex_colors_from_tess_3d_wire(&mut self, wire_indexes: &Vec<Integer>) {
        // TODO
        let mut wires: Vec<Vec<u32>> = vec![];

        let mut i = 0;
        while i < wire_indexes.len() {
            if wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsContinuous as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsContinuous not implemented!");
            }
            if wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsClosing as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsClosing not implemented!");
            }
            let number_of_indices_per_wire_edge = wire_indexes[i].value as u32 & 0x0FFFFFFF;
            //debug!("number of indices_per_wire_edge: {}", number_of_indices_per_wire_edge);
            wires.push(vec![]);

            let start = i + 1;
            for j in 0..number_of_indices_per_wire_edge {
                let id = start + j as usize;
                wires
                    .last_mut()
                    .unwrap()
                    .push(wire_indexes[id].value as u32);
                i += 1;
            }

            i += 1;
        }

        self.VertexColors_number_of_colors = 0;
        for w in wires {
            self.VertexColors_number_of_colors += w.len() as u32;
        }
    }

    pub fn TESS_3D_Wire__enter(&mut self) {
        self.TESS_3D_Wire_inside = true;
    }
    pub fn TESS_3D_Wire__leave(&mut self) {
        self.TESS_3D_Wire_inside = false;
    }
    pub fn TESS_3D_Wire__is_inside(&self) -> bool {
        self.TESS_3D_Wire_inside
    }

    /// group___tf_face_tess_data_____serialize_content2.html
    /// Note that the number of colors is deduced from the number of point indices as calculated from sizes_triangulated (in the preceding example, this would be 38) * 3 or 4 (RBG or RGBA).
    pub fn set_num_vertex_colors_from_tess_3d_face(
        &mut self,
        used_entities_flag: u32,
        triangulateddata: &Vec<UnsignedInteger>,
    ) {
        // TODO

        let mut num_colors_per_triangle = 0u32;
        if used_entities_flag != PrcTesselationFlags::PRC_FACETESSDATA_Triangle as u32 {
            warn!(
                "Only PRC_FACETESSDATA_Triangle is implemented! VertexColors_number_of_colors will be off!"
            );
        }
        num_colors_per_triangle = 3;

        self.VertexColors_number_of_colors = 0;
        for i in 0..triangulateddata.len() {
            self.VertexColors_number_of_colors +=
                num_colors_per_triangle * triangulateddata[i].value;
        }
    }

    //pub fn VertexColors_

    pub fn AnaFaceTrimLoop_start_new_loop(&mut self) {
        debug!("NEW LOOP");
        self.AnaFaceTrimLoop_curves.clear();
    }
    pub fn AnaFaceTrimLoop_add_curve_to_loop(&mut self, ref_or_cc: RefOrCompressedCurve) {
        if ref_or_cc.curve_is_not_already_stored.value {
            debug!(
                "CURVE TO LOOP: ADDING: {:?}",
                ref_or_cc.compressed_curve.as_ref().unwrap().id_concrete
            );
            self.AnaFaceTrimLoop_curves
                .push(ref_or_cc.compressed_curve.unwrap());
        } else {
            // TODO look up referenced curve
            let index = ref_or_cc.index_compressed_curve.as_ref().unwrap().value;
            let index_str;
            if index < self.AnaFaceTrimLoop_curves.len() as u32 {
                index_str = "valid".to_string();
            } else {
                index_str = "invalid".to_string();
            }
            debug!("CURVE TO LOOP: -> CURVE REF: {} ({})", index, index_str);
        }
    }
    //    pub fn AnaFaceTrimLoop_add_curve_to_loop1(&mut self, crv: CompressedCurve) {
    //        self.AnaFaceTrimLoop_curves.push(crv);
    //    }
    pub fn AnaFaceTrimLoop_store_loop(&mut self) {
        debug!("STORE LOOP");
        self.AnaFaceTrimLoop_loops
            .push(self.AnaFaceTrimLoop_curves.clone());
        self.AnaFaceTrimLoop_curves.clear();
    }
    //pub fn store_compressed_curve(&mut self, crv: CompressedCurve) {
    //}
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
    i: usize,
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

    ctx.prc_parsed.fsi[i].schema = Schema::from_reader(&mut reader, ctx).unwrap();
    let schema_data = &ctx.prc_parsed.fsi[i].schema;
    let _schema_str = format!("{:#?}", schema_data);
    if verbose {
        debug!("{}", _schema_str);
    }

    ctx.se = SchemaEvaluator::new(&schema_data.schemas);

    if parse_globals {
        ctx.prc_parsed.fsi[i].globals =
            PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut reader, ctx).unwrap();
        let data = &ctx.prc_parsed.fsi[i].globals;
        let _str = format!("{:#?}", data);
        if verbose {
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

fn prc_parse_tree(bytes: &Vec<u8>, i: usize, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_tree {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    ctx.prc_parsed.fsi[i].tree =
        PRC_TYPE_ASM_FileStructureTree::from_reader(&mut reader, ctx).unwrap();
    let data = &ctx.prc_parsed.fsi[i].tree;
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

fn prc_parse_tess(bytes: &Vec<u8>, i: usize, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_tess {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    ctx.prc_parsed.fsi[i].tess =
        PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut reader, ctx).unwrap();
    let data = &ctx.prc_parsed.fsi[i].tess;
    let _str = format!("{:#?}", data);
    if verbose {
        debug!("{}", _str);
    }

    let total_bits = (bytes.len() * 8) as u64;
    let consumed_bits = reader.position_in_bits().unwrap();
    let remaining_bits = total_bits - consumed_bits;
    debug!(
        "--tess ENDOK remaining: {} bits ({:.0}%), consumed bits: {} ({:.0}%) of {} ({} bytes) [took {} ms]--",
        remaining_bits,
        remaining_bits as f64 * 100.0 / total_bits as f64,
        consumed_bits,
        consumed_bits as f64 * 100.0 / total_bits as f64,
        total_bits,
        bytes.len(),
        now.elapsed().as_millis()
    );
    ()
}

fn prc_parse_geom(bytes: &Vec<u8>, i: usize, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_geom {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    ctx.prc_parsed.fsi[i].geom =
        PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut reader, ctx).unwrap();
    let data = &ctx.prc_parsed.fsi[i].geom;
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

fn prc_parse_extgeom(bytes: &Vec<u8>, i: usize, ctx: &mut PrcParsingContext, verbose: bool) {
    debug!(
        "--prc_parse_extgeom {} bits ({} bytes)--",
        bytes.len() * 8,
        bytes.len()
    );
    let now = std::time::Instant::now();

    let slice_of_u8 = bytes.as_slice();
    let mut reader = BitReader::endian(Cursor::new(slice_of_u8), bitstream_io::BigEndian);

    ctx.prc_parsed.fsi[i].ext =
        PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut reader, ctx).unwrap();
    let data = &ctx.prc_parsed.fsi[i].ext;
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

    ctx.prc_parsed.mf_schema = Schema::from_reader(&mut reader, ctx).unwrap();
    let schema_data = &ctx.prc_parsed.mf_schema;
    let _schema_str = format!("{:#?}", schema_data);
    if verbose {
        //debug!("{}", _schema_str);
    }

    ctx.prc_parsed.mf = PRC_TYPE_ASM_ModelFile::from_reader(&mut reader, ctx).unwrap();
    let data = &ctx.prc_parsed.mf;
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
    bytes: Vec<u8>,
    verbose: bool,
    all: bool,
    globals: bool,
    tree: bool,
    tess: bool,
    geom: bool,
    extgeom: bool,
    _schema: bool,
    modelfile: bool,
) -> io::Result<ParsedPrc> {
    debug_time!("prc_describe");

    let file_size_bytes = bytes.len();
    debug!("given {} bytes", file_size_bytes);
    let mut mem_reader: Cursor<Vec<u8>> = Cursor::new(bytes);

    let config = PRCHeader::from_reader(&mut mem_reader, file_size_bytes);
    let header = &config?;

    let mut ctx: PrcParsingContext = Default::default();
    ctx.ver_for_reading = header.verread;
    ctx.ver_authoring = header.verauth;

    ctx.filestructure_count = header.fsi.len() as u32;
    ctx.prc_parsed.verread = header.verread;
    ctx.prc_parsed.verauth = header.verauth;
    ctx.prc_parsed.uuid0 = header.uuid0;
    ctx.prc_parsed.uuid1 = header.uuid1;
    ctx.prc_parsed.uuid2 = header.uuid2;
    ctx.prc_parsed.uuid3 = header.uuid3;
    ctx.prc_parsed.uuida0 = header.uuida0;
    ctx.prc_parsed.uuida1 = header.uuida1;
    ctx.prc_parsed.uuida2 = header.uuida2;
    ctx.prc_parsed.uuida3 = header.uuida3;
    ctx.prc_parsed.fsi = Vec::with_capacity(header.fsi.len());
    ctx.prc_parsed
        .fsi
        .resize(header.fsi.len(), Default::default());

    // parse uncompressed files
    // TODO: could be processed concurrently
    // TODO: is it possible to use multiple,cloned contexts and merge them later?
    for i in 0..header.fsi.len() {
        debug_time!("--fsi={}--", i);
        ctx.prc_parsed.fsi[i].uuid0 = header.fsi[i].uuid0;
        ctx.prc_parsed.fsi[i].uuid1 = header.fsi[i].uuid1;
        ctx.prc_parsed.fsi[i].uuid2 = header.fsi[i].uuid2;
        ctx.prc_parsed.fsi[i].uuid3 = header.fsi[i].uuid3;
        prc_parse_globals(
            &header.fsi[i].sections[PrcSectionKind::Global as usize],
            i,
            &mut ctx,
            verbose,
            all || globals,
        );
        if all || tree {
            prc_parse_tree(
                &header.fsi[i].sections[PrcSectionKind::Tree as usize],
                i,
                &mut ctx,
                verbose,
            );
        }
        if all || tess {
            prc_parse_tess(
                &header.fsi[i].sections[PrcSectionKind::Tessellation as usize],
                i,
                &mut ctx,
                verbose,
            );
        }
        if all || geom {
            prc_parse_geom(
                &header.fsi[i].sections[PrcSectionKind::Geometry as usize],
                i,
                &mut ctx,
                verbose,
            );
        }
        if all || extgeom {
            prc_parse_extgeom(
                &header.fsi[i].sections[PrcSectionKind::ExtraGeometry as usize],
                i,
                &mut ctx,
                verbose,
            );
        }
    }
    if all || modelfile {
        // parse model file
        prc_parse_modfile(&header.mf, &mut ctx, verbose);
    }

    /*debug!(
        "PRC of {} bytes -> parsed+encoded into {:#?} bytes",
        file_size_bytes,
        ctx.prc_parsed.encode_into().len()
    );*/

    Ok(ctx.prc_parsed)
}

pub fn prc_describe_file(
    fname: &std::string::String,
    verbose: bool,
    all: bool,
    globals: bool,
    tree: bool,
    tess: bool,
    geom: bool,
    extgeom: bool,
    _schema: bool,
    modelfile: bool,
) -> io::Result<()> {
    debug_time!("prc_describe_file");

    // Create a path to the desired file
    let path = Path::new(fname);
    let display = path.display();

    info!("--parsing \"{}\"--", display);

    let mut now = std::time::Instant::now();
    // Open the path in read-only mode, returns `io::Result<File>`
    let mut _file = match File::open(&path) {
        Err(why) => return Err(why),
        Ok(file) => file,
    };

    let bytes: Vec<u8> = std::fs::read(fname)?;
    debug!("read {} bytes", bytes.len());
    debug!(
        "Reading into memory [took {} ms]",
        now.elapsed().as_millis()
    );

    let rv = prc_describe(
        bytes, verbose, all, globals, tree, tess, geom, extgeom, _schema, modelfile,
    );

    info!("--parsed successfully \"{}\"--", display);

    match rv {
        Err(why) => return Err(why),
        Ok(_data) => Ok(()),
    }
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
        let _ = prc_dump(&base, &a.fsi[i].sections[PrcSectionKind::Global as usize]);
        let base = base_name.replace(".prc", format!("_{i}_tree.bin").as_str());
        let _ = prc_dump(&base, &a.fsi[i].sections[PrcSectionKind::Tree as usize]);
        let base = base_name.replace(".prc", format!("_{i}_tess.bin").as_str());
        let _ = prc_dump(
            &base,
            &a.fsi[i].sections[PrcSectionKind::Tessellation as usize],
        );
        let base = base_name.replace(".prc", format!("_{i}_geom.bin").as_str());
        let _ = prc_dump(&base, &a.fsi[i].sections[PrcSectionKind::Geometry as usize]);
        let base = base_name.replace(".prc", format!("_{i}_extg.bin").as_str());
        let _ = prc_dump(
            &base,
            &a.fsi[i].sections[PrcSectionKind::ExtraGeometry as usize],
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn add(left: u64, right: u64) -> u64 {
        left + right
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    /// Read whole file into memory.
    fn get_file_as_byte_vec(filename: &std::string::String) -> Vec<u8> {
        let mut f = File::open(&filename).expect("no file found");
        let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("buffer overflow");

        buffer
    }

    #[test]
    fn test_describe() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[test_describe] The current directory is {}",
            path.display()
        );
        let bytes_external = get_file_as_byte_vec(&std::string::String::from(
            "testdata/pmi_sample.stream-23.prc",
        ));
        assert_eq!(bytes_external.len(), 24535usize);

        let parsed = prc_describe(
            bytes_external,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        );
        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();
        assert_eq!(parsed.verread, 7094);
        assert_eq!(parsed.verauth, 7094);
        assert_eq!(parsed.fsi.len(), 1);
        assert_eq!(parsed.fsi[0].tess.tess_count.value, 55);
        assert_eq!(parsed.fsi[0].tess.user_data.data.len(), 1usize);
        assert_eq!(parsed.mf.units_in_mm.value, 1.0);
        assert_eq!(parsed.mf.user_data.data.len(), 297usize);
    }
}
