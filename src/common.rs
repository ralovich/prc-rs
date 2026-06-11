// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(non_snake_case)]
#![allow(unused)]

use crate::builtin::*;
use crate::constants::*;
use crate::indent;
use crate::prc_gen::*;
use crate::schema::SchemaEvaluator;
use crate::tess_3d_compressed::Tess3dCompressed;
use log::{debug, error, info, trace, warn};
use measure_time::debug_time;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::Cursor;
use std::path::Path;
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ParsedPrcFileStructure {
    pub uuid: [u32; 4],
    pub header: UncompressedFileStructureHeader,
    pub schema: Schema,
    pub glob: PRC_TYPE_ASM_FileStructureGlobals,
    pub tree: PRC_TYPE_ASM_FileStructureTree,
    pub tess: PRC_TYPE_ASM_FileStructureTessellation,
    pub geom: PRC_TYPE_ASM_FileStructureGeometry,
    pub extg: PRC_TYPE_ASM_FileStructureExtraGeometry,
}

/// All information from a parsed PRC. Can be (de-)serialized into e.g. JSON.
/// TODO: merge with PrcHeader?
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct ParsedPrc {
    pub verread: u32,
    pub verauth: u32,
    pub uuid_file: [u32; 4],
    pub uuid_application: [u32; 4],
    pub fsi: Vec<ParsedPrcFileStructure>,
    pub mf_schema: Schema,
    pub mf: PRC_TYPE_ASM_ModelFile,
    pub uncompr_files: Vec<Vec<u8>>,
}
impl ParsedPrc {
    pub fn uncompr_files_size(&self) -> u32 {
        let mut num_bytes = 0;
        for i in 0..self.uncompr_files.len() {
            num_bytes += self.uncompr_files[i].len() as u32;
        }
        num_bytes
    }
}

/// See also [prc_rs::prc_gen::PRC_TYPE_TESS_3D_Wire]
#[derive(Default, Clone)]
pub struct Tess3dWire {
    coordinates: Vec<Double>,
    wire_indexes: Vec<Integer>,
    is_segment_color: bool,
    VertexColors_number_of_colors: u32,
}
impl Tess3dWire {
    pub fn set0(&mut self, coordinates: &Vec<Double>, wire_indexes: &Vec<Integer>) {
        self.coordinates = coordinates.clone();
        self.wire_indexes = wire_indexes.clone();
    }
    pub fn set1(&mut self, is_segment_color: bool) {
        self.is_segment_color = is_segment_color;
    }
    /// group___tf3_d_wire_tess_data_____serialize_content2.html
    /// Note that the number of colors is deduced from the number of point indices as calculated from wire_indexes * 3 or 4 (RGB or RGBA).
    /// It is important to remember that implicit points must also have a color (see Special flags for 3DWireTessData tessellation).
    pub fn get_num_vertex_colors(&mut self) -> u32 {
        // TODO
        let mut wires: Vec<Vec<u32>> = vec![];

        if self.wire_indexes.is_empty() {
            // If number_of_wire_indexes is zero, the tessellation is given as a single wire edge containing an array of points as described in SerializeContentBaseTessData.
        } else {
        }

        // if self.TESS_3D_Wire_inside {
        //
        // }

        let mut i = 0;
        while i < self.wire_indexes.len() {
            if self.wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsContinuous as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsContinuous not implemented!");
            }
            if self.wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsClosing as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsClosing not implemented!");
            }
            // The flag is the leftmost 4 bits and is interpreted using 3D Wire Tess Flags to indicate
            let number_of_indices_per_wire_edge = self.wire_indexes[i].value as u32 & 0x7FFFFFFF;
            debug!(
                "number of indices_per_wire_edge: {}",
                number_of_indices_per_wire_edge
            );
            wires.push(vec![]);

            let start = i + 1;
            for j in 0..number_of_indices_per_wire_edge {
                let id = start + j as usize;
                wires
                    .last_mut()
                    .unwrap()
                    .push(self.wire_indexes[id].value as u32);
                i += 1;
            }

            i += 1;
        }

        self.VertexColors_number_of_colors = 0;
        for w in wires {
            if !self.is_segment_color {
                self.VertexColors_number_of_colors += w.len() as u32;
            } else {
                self.VertexColors_number_of_colors += w.len() as u32 - 1;
            }
        }
        // if self.VertexColors_number_of_colors == 62 {
        //     self.VertexColors_number_of_colors = 60;
        // }
        // if self.VertexColors_number_of_colors == 8 {
        //     self.VertexColors_number_of_colors = 6;
        // }

        debug!(
            "VertexColors_number_of_colors: {}",
            self.VertexColors_number_of_colors
        );
        return self.VertexColors_number_of_colors;
    }
}

#[derive(Debug, Default, Clone)]
pub struct ComprNurbs {
    pub degree_in_u: u32,
    pub degree_in_v: u32,
    pub number_bits_u: u32,
    pub number_bits_v: u32,
    pub number_ccpt_in_u: u32, // Number_ccpt_in_u = sumOf(mult_u) - degree_in_u - 1
    //Number_ccpt_in_v = sumOf(mult_v) - degree_in_v - 1
    pub number_ccpt_in_v: u32,
    pub number_stored_knots_in_u: u32,
    pub number_stored_knots_in_v: u32,
    pub number_of_knots_in_u: u32,
    pub number_of_knots_in_v: u32,
    is_closed_in_u: bool,
    is_closed_in_v: bool,
    number_of_bits_for_isomin: u32,
    number_of_bits_for_rest: u32,

    mult_u_flat: Vec<u32>,
    mult_v_flat: Vec<u32>,
    knots_u_type_param: u32,
    knots_u: Vec<f64>,
}
impl ComprNurbs {
    pub fn set0(&mut self, degree_in_u: u32, degree_in_v: u32, number_stored_knots_in_u: u32) {
        self.degree_in_u = degree_in_u;
        self.number_bits_u = if degree_in_u != 0 {
            ((degree_in_u as f64 + 2.0).ln() / (2.0_f64).ln()).ceil() as u32
        } else {
            2_u32
        };
        self.degree_in_v = degree_in_v;
        self.number_bits_v = if degree_in_v != 0 {
            ((degree_in_v as f64 + 2.0).ln() / (2.0_f64).ln()).ceil() as u32
        } else {
            2_u32
        };
        self.number_stored_knots_in_u = number_stored_knots_in_u;
        self.number_of_knots_in_u = number_stored_knots_in_u + 2;
    }
    pub fn set1(&mut self, mult_u: &Vec<CompressedMultiplicitiesU>, number_stored_knots_in_v: u32) {
        let sum_u;
        (self.mult_u_flat, sum_u) = sum_up_u(mult_u);
        self.number_ccpt_in_u = sum_u - self.degree_in_u - 1;
        self.number_stored_knots_in_v = number_stored_knots_in_v;
        self.number_of_knots_in_v = number_stored_knots_in_v + 2;
    }
    pub fn set2(&mut self, mult_v: &Vec<CompressedMultiplicitiesV>) {
        let sum_v;
        (self.mult_v_flat, sum_v) = sum_up_v(&mult_v);
        self.number_ccpt_in_v = sum_v - self.degree_in_v - 1;
    }
    pub fn set3(
        &mut self,
        is_closed_in_u: bool,
        is_closed_in_v: bool,
        number_of_bits_for_isomin: u32,
        number_of_bits_for_rest: u32,
    ) {
        self.is_closed_in_u = is_closed_in_u;
        self.is_closed_in_v = is_closed_in_v;
        self.number_of_bits_for_isomin = number_of_bits_for_isomin;
        self.number_of_bits_for_rest = number_of_bits_for_rest;
        debug!("{:#?}", self);
    }
    pub fn set4(&mut self, ccpt: &CompressedControlPoints) {
        let mut cp: Vec<Vec<[f64; 3]>> =
            vec![
                vec![Default::default(); self.number_ccpt_in_v as usize];
                self.number_ccpt_in_u as usize
            ];
        cp[0][0] = [ccpt.p00.x.value, ccpt.p00.y.value, ccpt.p00.z.value];
        for i in 0..ccpt.ccpt_in_u.len() {
            // FIXME:
            cp[i + 1][0] = [
                cp[i][0][0] + ccpt.ccpt_in_u[i].x,
                cp[i][0][1] + ccpt.ccpt_in_u[i].y,
                cp[i][0][2] + ccpt.ccpt_in_u[i].z,
            ];
        }
        for j in 0..ccpt.ccpt_in_v.len() {
            // FIXME:
            cp[0][j + 1] = [
                cp[0][j][0] + ccpt.ccpt_in_v[j].x,
                cp[0][j][1] + ccpt.ccpt_in_v[j].y,
                cp[0][j][2] + ccpt.ccpt_in_v[j].z,
            ];
        }
        fn get_interior_pt(
            ccpt: &CompressedControlPoints,
            nu: usize,
            nv: usize,
            u: usize,
            v: usize,
        ) -> [f64; 3] {
            assert!(nu > 1);
            assert!(nv > 1);
            let id = (nv - 1) * u + v;
            trace!("id={}", id);
            let inpt = &ccpt.ccpt_interior[id];
            match inpt._type.value {
                0 => [0.0, 0.0, 0.0], // FIXME:
                1 => [0.0, 0.0, inpt.p1z.unwrap().value],
                2 => [inpt.p2x.unwrap().value, inpt.p2y.unwrap().value, 0.0],
                3 => [
                    inpt.p3x.unwrap().value,
                    inpt.p3y.unwrap().value,
                    inpt.p3z.unwrap().value,
                ],
                _ => unreachable!("v={}", inpt._type.value),
            }
        }
        for u in 1..self.number_ccpt_in_u as usize {
            for v in 1..self.number_ccpt_in_v as usize {
                let pt = get_interior_pt(
                    ccpt,
                    self.number_ccpt_in_u as usize,
                    self.number_ccpt_in_v as usize,
                    u - 1,
                    v - 1,
                );
                cp[u][v] = pt;
            }
        }

        debug!("{:#?}", cp);
    }
    pub fn set5(&mut self, knot_vector_u: &CompressedKnotVectorU) {
        //debug!("{:#?}", knot_vector_u);
        let mut knots = vec![];
        let type_param: u32;
        if !knot_vector_u.is_uniform {
            if knot_vector_u.knots.as_ref().unwrap().is_unknown_form.value {
                type_param = 1;
                for knot in knot_vector_u
                    .knots
                    .as_ref()
                    .unwrap()
                    .compressed_knots
                    .iter()
                {
                    let knot_value;
                    if knot_vector_u
                        .knots
                        .as_ref()
                        .unwrap()
                        .number_bit_parameter
                        .value
                        > 30
                    {
                        knot_value = knot.knot.unwrap().value;
                    } else {
                        knot_value = knot.knot_vbr.unwrap().value;
                    }
                    assert!(knot_value >= 0.0);
                    assert!(knot_value <= 1.0);
                    knots.push(knot_value);
                }
            } else {
                type_param = 2;
            }
        } else {
            type_param = 0;
        }
        debug!("U knots {:#?}", (type_param, &knots));
        self.knots_u_type_param = type_param;
        self.knots_u = knots;
    }
}

#[derive(Default, Clone)]
pub struct PrcParsingContext {
    pub ver_for_reading: u32,
    pub ver_authoring: u32,

    pub current_name: std::string::String,
    pub layer_index: u32,
    pub index_of_line_style: u32,
    pub behavior_bit_field: u16,
    pub se: SchemaEvaluator,

    //pub is_an_iso_face: bool, // ContentCompressedFace
    pub ContentCurve_is_3d_flag: bool,
    pub PRC_TYPE_CRV_NURBS_is_rational: bool,

    //pub number_of_bits_to_store_reference: u32,
    /// ISO 2014: is TRUE if this compressed line is part of a PRC_TYPE_TOPO_BrepDataCompress; it is FALSE if this compressed line is a part of a PRC_TYPE_TOPO_SingleWireBodyCompress.
    /// Acrobat SDK 9: group___tf_x_k_circle_____serialize.html
    /// curve_trimming_face indicates that this function is called by the Serialize ContentCompressedFace function.
    /// PRC_TYPE_TOPO_SingleWireBodyCompress: curve_trimming_face is always FALSE in a single wire context.
    curve_trimming_face: bool,

    /// ISO 2014: is TRUE if the circle is being used as the trim boundary of an PRC_HCG_IsoNurbs; otherwise it is FALSE
    /// Acrobat SDK 9: group___tf_x_k_circle_____serialize.html
    /// compressed_iso_spline_serialization is true if this function is called from SerializeCompressedIsoNurbs.
    compressed_iso_spline: bool,
    //surface_type: u32, // PRC_HCG_...
    current_face_type: Vec<CompressedEntityType>, // stack of PRC_HCG_...

    /// iso-vertices (iso-cache) // https://github.com/pdf-association/pdf-issues/issues/705#issuecomment-3697983893
    /// ana-vertices (ana-cache)
    /// number_vertex_references  is just a portion of the referenced vertices. Technically,
    /// it is a number of referenced iso-vertices, i.e. vertices referenced from iso faces.
    /// In simple words, imagine there are two caches for compressed vertices - one for
    /// iso-vertices (iso-cache), and one for ana-vertices (ana-cache). When a compressed vertex
    /// is referenced by point_index, for iso-vertex get vertex from iso-cache by point_index,
    /// and for ana-vertex get vertex from ana-cache using (point_index - number_vertex_references)
    /// as an index.
    pub BrepDataCompress_number_vertex_references: u32,
    pub BrepDataCompress_number_edge_references: u32,
    BrepDataCompress_sum_num_faces: u32,
    #[allow(non_snake_case)]
    pub BrepDataCompress_number_of_bits_to_store_reference: u32,

    pub brep_data_compressed_tolerance: f64,
    pub nurbs_tolerance: f64, /*= self.brep_data_compressed_tolerance / 5.0*/
    // used for (Interior)CompressedControlPoints
    pub CompressedNurbs_number_stored_knots_in_u: u32, /*= self.number_of_knots_in_u ‐ 2*/
    pub CompressedNurbs_number_stored_knots_in_v: u32, /*= self.number_of_knots_in_v – 2*/
    pub CompressedNurbs_number_bits_u: u32, /*= (degree_in_u ? ceil[ log( degree_in_u + 2 ) / log(2) ] : 2)*/
    pub CompressedNurbs_number_bits_v: u32, /*= (degree_in_v ? ceil[ log( degree_in_v + 2 ) / log(2) ] : 2)*/
    pub CompressedKnots_tolerance_parameter: f64, /*= 1./ 2^( number_bit_parameter ‐1)*/
    //pub CompressedNurbs_number_ccpt_in_u: u32, // Number_ccpt_in_u = sumOf(mult_u) - degree_in_u - 1
    //Number_ccpt_in_v = sumOf(mult_v) - degree_in_v - 1
    //pub CompressedNurbs_number_ccpt_in_v: u32, // CompressedControlPoints https://github.com/pdf-association/pdf-issues/issues/663
    pub CompressedKnots_number_bit_parameter: u32, // the number of bits used to store knots
    pub CompressedNurbs_number_of_bits_for_isomin: u32, // number of bits used to store first row and column of control points
    pub CompressedNurbs_number_of_bits_for_rest: u32, // number of bits to store the remainder of the control points
    pub compressed_nurbs: ComprNurbs,

    pub VertexColors_number_of_colors: u32,
    pub VertexColors_is_segment_color: bool,

    /// 8.9.21.10 CompressedVertex
    /// Each compressed brep data serialization maintains an array of previously written
    /// vertices, starting at index 0.
    pub BrepDataCompress_CompressedVertex_array: Vec<Vec<CompressedPoint>>,

    /// TODO: scope is per BrepDataCompress?
    /// comes from RefOrCompressedCurve
    /// The flag curve_is_not_already_stored indicates if the trim curve has already been stored in the
    /// compressed brep data. If the curve has already been stored, the index of the curve is stored in the file;
    /// otherwise, a compressed version of the trim curve is stored.
    /// the current loop
    pub AnaFaceTrimLoop_curves: Vec<RefOrCompressedCurve>,
    /// accumulator of loops
    pub AnaFaceTrimLoop_loops: Vec<Vec<RefOrCompressedCurve>>,

    /// Whether the parser is progressing within a PRC_TYPE_TESS_3D_Wire struct?
    TESS_3D_Wire_inside: bool,
    pub t3dw: Tess3dWire,
    pub t3dc: Tess3dCompressed,

    ContentCompressedFace_owner_is_an_iso_face: Option<bool>,

    pub prc_parsed: ParsedPrc,
}
impl PrcParsingContext {
    /// See ContentCompressedFace in spec.
    ///
    /// Vertex loops are used to represent a loop consisting of a single
    /// vertex, such as might exist on the apex of a cone, or a sphere
    /// touching a plane. They are represented by a degenerate line which
    /// has identical start and end vertices.
    pub fn ContentCompressedFace_all_loops_are_vertex_loops(&self) -> bool {
        fn distance(start: &CompressedPoint, end: &CompressedPoint) -> f64 {
            let dx = start.x - end.x;
            let dy = start.y - end.y;
            let dz = start.z - end.z;
            let d = (dx * dx + dy * dy + dz * dz).sqrt();
            trace!("EVAL DISTANCE {}", d);
            d
        }

        // TODO
        //warn!("FIXME: all_loops_are_vertex_loops: not implemented yet");

        // from https://github.com/pdf-association/pdf-issues/issues/696#issuecomment-3599474152
        // Which means that ALL the loops in this array of loops are degenerate lines (a line with two
        // points that are the same with precision/tolerance), the surface type is PRC_HCG_AnaTorus and
        // is_trimmed is TRUE.

        let tol = self.brep_data_compressed_tolerance / 100.0;
        let mut all_loops_are_vertex_loops = true;

        for _loop in self.AnaFaceTrimLoop_loops.iter() {
            if _loop.len() > 1 {
                all_loops_are_vertex_loops = false;
                break;
            }
            for curve in _loop {
                // not a curve but a ref
                if !curve.curve_is_not_already_stored {
                    all_loops_are_vertex_loops = false;
                    break;
                }
                //let id = curve.compressed_curve.as_ref().unwrap().id_concrete;
                match curve.compressed_curve.as_ref().unwrap().id_concrete {
                    CompressedCurve_idConcrete::line(l) => {
                        if l.start_end_data.start_point.is_some() {
                            if distance(
                                &l.start_end_data.start_point.unwrap(),
                                &l.start_end_data.end_point.unwrap(),
                            ) >= tol
                            {
                                all_loops_are_vertex_loops = false;
                                break;
                            }
                        } else {
                            assert!(l.start_end_data.start_vertex.is_some());
                            let sv = &l.start_end_data.start_vertex.unwrap();
                            let s = self.BrepDataCompress_CompressedVertex_get2(sv);
                            let ev = &l.start_end_data.end_vertex.unwrap();
                            let e = self.BrepDataCompress_CompressedVertex_get2(ev);
                            if distance(&s, &e) >= tol {
                                all_loops_are_vertex_loops = false;
                                break;
                            }
                        }
                    }
                    _ => {
                        all_loops_are_vertex_loops = false;
                        break;
                    }
                }
            }
        }

        if all_loops_are_vertex_loops {
            error!("RARE CASE: all_loops_are_vertex_loops=true !!");
        }

        all_loops_are_vertex_loops
    }

    pub fn ContentCompressedAnaFace_has_point_on_torus(&self, is_trimmed: bool) -> bool {
        let all_loops_are_vertex_loops = self.ContentCompressedFace_all_loops_are_vertex_loops();

        let st = self.get_surface_type();
        if st.is_some() && st.unwrap().value == PrcCompressedFaceType::PRC_HCG_AnaTorus as u8 {
            let surface_type = st.unwrap();
            error!(
                "surface_type: {:?}, is_trimmed: {} -> all_loops_are_vertex_loops should probably return TRUE",
                surface_type, is_trimmed
            );
        }

        let has = all_loops_are_vertex_loops
            && self.get_surface_type().unwrap().value
                == PrcCompressedFaceType::PRC_HCG_AnaTorus as u8
            && is_trimmed;
        return has;
    }

    pub fn ContentCompressedFace_owner_enter(&mut self, is_an_iso_face: bool) {
        assert!(self.ContentCompressedFace_owner_is_an_iso_face.is_none());
        self.ContentCompressedFace_owner_is_an_iso_face = Some(is_an_iso_face);
    }
    pub fn ContentCompressedFace_owner_leave(&mut self) {
        self.ContentCompressedFace_owner_is_an_iso_face = None;
    }
    pub fn ContentCompressedFace_owner_is_an_iso_face(&self) -> bool {
        self.ContentCompressedFace_owner_is_an_iso_face.unwrap()
    }

    pub fn push_face_type(&mut self, cet: CompressedEntityType) {
        let id = cet.value;
        self.current_face_type.push(cet);
        warn!("Pushing face {:?} [{}]", cet, self.current_face_type.len());
    }
    pub fn pop_face_type(&mut self) {
        assert!(!self.current_face_type.is_empty());
        let face = self.current_face_type[self.current_face_type.len() - 1];
        self.current_face_type.pop();
        if self.current_face_type.is_empty() {
            warn!("Popping face {:?}, NO new top! [0]", face);
        } else {
            let new_top = self.current_face_type[self.current_face_type.len() - 1];
            warn!(
                "Popping face {:?}, new top is {:?} [{}]",
                face,
                new_top,
                self.current_face_type.len()
            );
        }
    }
    pub fn get_surface_type(&self) -> Option<CompressedEntityType> {
        if self.current_face_type.is_empty() {
            return None;
        }
        Some(self.current_face_type[self.current_face_type.len() - 1])
    }

    //pub fn on_brep_data_compress(&mut self, _bdc: &PRC_TYPE_TOPO_BrepDataCompress) {
    //    self.nurbs_tolerance = self.brep_data_compressed_tolerance / 5.0;
    //    //self.number_stored_knots_in_u = bdc.number_of_knots_in_u ‐ 2;
    //    panic!("Not implemented!");
    //}

    pub fn BrepDataCompress_enter(&mut self) {
        self.BrepDataCompress_CompressedVertex_array
            .push(Vec::new());
        self.set_curve_trimming_face(true);
        self.BrepDataCompress_sum_num_faces = 0;
    }
    pub fn BrepDataCompress_leave(&mut self) {
        assert!(!self.BrepDataCompress_CompressedVertex_array.is_empty());
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        debug!(
            "BrepDataCompress_leave: {} vertices",
            self.BrepDataCompress_CompressedVertex_array[last_idx].len()
        );
        self.set_curve_trimming_face(false);
        self.BrepDataCompress_number_of_bits_to_store_reference = 0;
        self.brep_data_compressed_tolerance = 0.0;
        self.nurbs_tolerance = 0.0;
    }
    pub fn BrepDataCompress_register_faces(&mut self, num_faces: u32) {
        let prev = self.BrepDataCompress_sum_num_faces;
        self.BrepDataCompress_sum_num_faces += num_faces;
        warn!(
            "{}BrepDataCompress_sum_num_faces: {} -> {}",
            indent::get(),
            prev,
            self.BrepDataCompress_sum_num_faces
        );
    }
    pub fn BrepDataCompress_get_sum_num_faces(&self) -> u32 {
        self.BrepDataCompress_sum_num_faces
    }
    pub fn BrepDataCompress_CompressedVertex_add(&mut self, pt: CompressedPoint) {
        if self.BrepDataCompress_CompressedVertex_array.is_empty() {
            warn!("BrepDataCompress_CompressedVertex_array is empty!");
            return;
        }
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        // FIXME: is this the right array to add to?
        self.BrepDataCompress_CompressedVertex_array[last_idx].push(pt);
    }
    pub fn BrepDataCompress_CompressedVertex_get(
        &self,
        point_index: u32,
    ) -> Option<CompressedPoint> {
        if self.BrepDataCompress_CompressedVertex_array.is_empty() {
            warn!("BrepDataCompress_CompressedVertex_array is empty!");
            return None;
        }
        let last_idx = self.BrepDataCompress_CompressedVertex_array.len() - 1;
        if point_index as usize >= self.BrepDataCompress_CompressedVertex_array[last_idx].len() {
            warn!("BrepDataCompress_CompressedVertex_array is too small!");
            return None;
        }
        // FIXME: is this the right array to index into?
        let pt = self.BrepDataCompress_CompressedVertex_array[last_idx][point_index as usize];
        //debug!("{}ref vtx: {} -> {:?}", indent::get(), point_index, pt);
        Some(pt)
    }
    pub fn BrepDataCompress_CompressedVertex_get2(&self, v: &CompressedVertex) -> CompressedPoint {
        if v.already_stored.value {
            v.point_data.unwrap()
        } else {
            self.BrepDataCompress_CompressedVertex_get(v.point_index.unwrap().value)
                .unwrap()
        }
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
        error!(
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
    pub fn set_num_vertex_colors_from_tess_3d_wire(
        &mut self,
        coordinates: &Vec<Double>,
        wire_indexes: &Vec<Integer>, /*is_segment_color: bool*/
    ) {
        // TODO
        let mut wires: Vec<Vec<u32>> = vec![];

        if wire_indexes.is_empty() {
            // If number_of_wire_indexes is zero, the tessellation is given as a single wire edge containing an array of points as described in SerializeContentBaseTessData.
        } else {
        }

        if self.TESS_3D_Wire_inside {}

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
            // The flag is the leftmost 4 bits and is interpreted using 3D Wire Tess Flags to indicate
            let number_of_indices_per_wire_edge = wire_indexes[i].value as u32 & 0x7FFFFFFF;
            debug!(
                "number of indices_per_wire_edge: {}",
                number_of_indices_per_wire_edge
            );
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
            if false
            /*is_segment_color*/
            {
                self.VertexColors_number_of_colors += w.len() as u32;
            } else {
                self.VertexColors_number_of_colors += w.len() as u32 - 1;
            }
        }
        // if self.VertexColors_number_of_colors == 62 {
        //     self.VertexColors_number_of_colors = 60;
        // }
        // if self.VertexColors_number_of_colors == 8 {
        //     self.VertexColors_number_of_colors = 6;
        // }
    }

    pub fn VertexColors_get_number_of_colors(&mut self) -> u32 {
        if self.TESS_3D_Wire_inside {
            return self.t3dw.get_num_vertex_colors();
        }
        self.VertexColors_number_of_colors
    }

    pub fn TESS_3D_Wire__enter(&mut self) {
        self.TESS_3D_Wire_inside = true;
        self.VertexColors_number_of_colors = 0;
        self.VertexColors_is_segment_color = false;
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
        if used_entities_flag != PrcTesselationFlags::PRC_FACETESSDATA_Triangle as u32
            && used_entities_flag != PrcTesselationFlags::PRC_FACETESSDATA_TriangleTextured as u32
        {
            warn!(
                "Only PRC_FACETESSDATA_Triangle and PRC_FACETESSDATA_TriangleTextured are implemented! VertexColors_number_of_colors will be off!"
            );
        }
        num_colors_per_triangle = 3;

        self.VertexColors_number_of_colors = 0;
        for i in 0..triangulateddata.len() {
            self.VertexColors_number_of_colors +=
                num_colors_per_triangle * triangulateddata[i].value;
        }
    }

    pub fn AnaFaceTrimLoop_start_new_loop(&mut self) {
        debug!("NEW LOOP");
        self.AnaFaceTrimLoop_curves.clear();
    }
    pub fn AnaFaceTrimLoop_add_curve_to_loop(&mut self, ref_or_cc: RefOrCompressedCurve) {
        if ref_or_cc.curve_is_not_already_stored.value {
            debug!(
                "CURVE TO LOOP: ADDING CURVE: {:?}",
                ref_or_cc.compressed_curve.as_ref().unwrap().id_concrete
            );
        } else {
            // TODO look up referenced curve
            let index = ref_or_cc.index_compressed_curve.as_ref().unwrap().value;
            let index_str;
            if index < self.AnaFaceTrimLoop_curves.len() as u32 {
                index_str = "valid".to_string();
            } else {
                index_str = "invalid".to_string();
            }
            debug!("CURVE TO LOOP: ADDING REF: {} ({})", index, index_str);
        }
        self.AnaFaceTrimLoop_curves.push(ref_or_cc);
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

    pub fn CompressedShell_reorder_faces(&mut self) {
        warn!("TODO: CompressedShell_reorder_faces not yet implemented!");
    }
}

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

    let mut ctx: PrcParsingContext = Default::default();
    let header = UncompressedFileHeader::from_reader(&mut mem_reader, &mut ctx)?;

    header.decompress_sections(
        &mut mem_reader,
        &mut ctx,
        file_size_bytes,
        verbose,
        all,
        globals,
        tree,
        tess,
        geom,
        extgeom,
        _schema,
        modelfile,
    )?;

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
    debug_time!("prc_describe_file \"{}\"", fname);

    // Create a path to the desired file
    let path = Path::new(fname);
    let display = path.display();

    let path = std::env::current_dir()?;
    info!("The current directory is {}", path.display());
    info!("--parsing \"{}\"--", display);

    let now = std::time::Instant::now();

    let bytes: Vec<u8> = std::fs::read(fname)?;
    debug!("read {} bytes", bytes.len());
    debug!(
        "Reading into memory [took {} ms]",
        now.elapsed().as_millis()
    );

    let rv = prc_describe(
        bytes, verbose, all, globals, tree, tess, geom, extgeom, _schema, modelfile,
    );

    match rv {
        Err(why) => {
            warn!("--parsing failed: {}", why);
            Err(why)
        }
        Ok(_data) => {
            info!("--parsed successfully \"{}\"--", display);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_common::tests::*;
    use std::io::Read;

    #[test]
    fn test_describe() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[{}] The current directory is {}",
            function!(),
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

    #[test]
    fn test_describe_compr() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[test_describe_compr] The current directory is {}",
            path.display()
        );
        let bytes_external = get_file_as_byte_vec(&std::string::String::from(
            "testdata/3D-PDF-Sample-School.stream-48.prc",
        ));
        assert_eq!(bytes_external.len(), 427862usize);

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
        assert_eq!(parsed.verauth, 15083);
        assert_eq!(parsed.fsi.len(), 1);
        assert_eq!(parsed.fsi[0].tess.tess_count.value, 348);
        assert_eq!(parsed.fsi[0].tess.user_data.data.len(), 1usize);
        assert_eq!(parsed.mf.units_in_mm.value, 1000.0);
        assert_eq!(parsed.mf.user_data.data.len(), 278usize);
    }
}
