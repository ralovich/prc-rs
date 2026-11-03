// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(non_snake_case)]
//#![allow(unused)]

use crate::builtin::*;
use crate::constants::*;
use crate::prc_gen::*;
use crate::schema::SchemaEvaluator;
use crate::tess_3d_compressed::Tess3dCompressed;
use crate::tess_3d_wire::Tess3dWire;
use crate::{LIBPRC_JSON_SCHEMA_VERSION, compressed_nurbs, indent};
use bitstream_io::BitReader;
use log::{debug, error, info, trace, warn};
use measure_time::debug_time;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::BufWriter;
use std::io::Write;
use std::io::{Cursor, SeekFrom};
use std::path::Path;

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

/// decompressed sections from binary PRC, not part of JSON serialization
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DecompressedSections {
    pub uuids: Vec<[u32; 4]>,
    pub sections_decompressed: Vec<[Vec<u8>; PrcSectionKind::Count as usize]>,
    pub mf_decompressed: Vec<u8>,
}

pub struct ParsedSections {
    pub fsi: Vec<ParsedPrcFileStructure>,
    pub mf_schema: Schema,
    pub mf: PRC_TYPE_ASM_ModelFile,
}

/// Flags that control parsing and optionally enable dumping attached files.
#[derive(Clone, Debug)]
pub struct ParsingOptions {
    pub verbose: bool,
    pub all: bool,
    pub globals: bool,
    pub tree: bool,
    pub tess: bool,
    pub geom: bool,
    pub extgeom: bool,
    pub schema: bool,
    pub modelfile: bool,
    pub dump_attachments: bool,
}

/// All information from a parsed PRC. Can be (de-)serialized into e.g. JSON.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ParsedPrc {
    /// these fields are already available before decompression
    pub json_schema_version: u32,
    pub verread: u32,
    pub verauth: u32,
    pub uuid_file: [u32; 4],
    pub uuid_application: [u32; 4],
    pub uncompr_files: Vec<Vec<u8>>,
    /// Decompressed sections from binary PRC, not part of JSON serialization.
    /// Used temporarily during loading.
    #[serde(skip)]
    pub decompressed_sections: Option<DecompressedSections>,

    /// these fields become available after decompression
    pub fsi: Vec<ParsedPrcFileStructure>,
    pub mf_schema: Schema,
    pub mf: PRC_TYPE_ASM_ModelFile,
}
impl Default for ParsedPrc {
    fn default() -> Self {
        Self {
            json_schema_version: LIBPRC_JSON_SCHEMA_VERSION,
            verread: 0,
            verauth: 0,
            uuid_file: [0; 4],
            uuid_application: [0; 4],
            uncompr_files: vec![],
            decompressed_sections: None,
            fsi: vec![],
            mf_schema: Default::default(),
            mf: Default::default(),
        }
    }
}
impl ParsedPrc {
    /// required when parsing ModelFile
    pub fn get_num_fsi(&self) -> usize {
        if let Some(dc) = &self.decompressed_sections {
            dc.sections_decompressed.len()
        } else {
            self.fsi.len()
        }
    }
    pub fn uncompressed_files_size(&self) -> u32 {
        let mut num_bytes = 0;
        for ufi in &self.uncompr_files {
            num_bytes += ufi.len() as u32;
        }
        num_bytes
    }
    pub fn parse_decompressed(
        &mut self,
        ctx: &mut PrcParsingContext,
        opts: &ParsingOptions,
    ) -> io::Result<()> {
        if self.decompressed_sections.is_none() {
            return Ok(());
        }
        let parsed_sections = UncompressedFileHeader::parse_decompressed_sections(
            self.decompressed_sections.as_ref().unwrap(),
            ctx,
            opts,
        )?;
        self.fsi = parsed_sections.fsi;
        self.mf_schema = parsed_sections.mf_schema;
        self.mf = parsed_sections.mf;
        self.decompressed_sections = None;

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct StructInfo {
    num: usize, // number of instances encountered
}
impl std::fmt::Display for StructInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Accumulates state during parsing.
#[derive(Default, Clone)]
pub struct PrcParsingContext {
    pub file_base_name: std::string::String,
    pub authoring_version: u32,
    pub num_fsi: usize,

    pub current_name: std::string::String,
    pub layer_index: u32,
    pub index_of_line_style: u32,
    pub behavior_bit_field: u16,
    pub se: SchemaEvaluator,

    struct_stats: std::collections::BTreeMap<std::string::String, StructInfo>,

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

    pub compressed_nurbs: compressed_nurbs::CompressedNurbs,

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
}
impl PrcParsingContext {
    pub fn set_authoring_version(&mut self, authoring_version: u32) {
        self.authoring_version = authoring_version;
    }
    /// required in many places during parsing
    pub fn get_authoring_version(&self) -> u32 {
        self.authoring_version
    }
    pub fn set_num_fsi(&mut self, num_fsi: usize) {
        self.num_fsi = num_fsi;
    }
    /// required by ModelFile during parsing
    pub fn get_num_fsi(&self) -> usize {
        self.num_fsi
    }

    pub fn on_read_begin(&mut self, struct_name: &str) {
        let s = struct_name.to_string();
        match self.struct_stats.get_mut(&s) {
            Some(v) => v.num += 1,
            None => {
                self.struct_stats.insert(s, StructInfo { num: 1 });
            }
        }
    }

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
                match curve.compressed_curve.as_ref().unwrap().id_concrete {
                    CompressedCurve_idConcrete::line(l) => {
                        if let Some(start_point) = l.start_end_data.start_point {
                            //if l.start_end_data.start_point.is_some() {
                            if distance(
                                &start_point,
                                //&l.start_end_data.start_point.unwrap(),
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
        if let Some(surface_type) = st
            && surface_type.value == PrcCompressedFaceType::PRC_HCG_AnaTorus as u8
        {
            //if st.is_some() && st.unwrap().value == PrcCompressedFaceType::PRC_HCG_AnaTorus as u8 {
            //let surface_type = st.unwrap();
            error!(
                "surface_type: {:?}, is_trimmed: {} -> all_loops_are_vertex_loops should probably return TRUE",
                surface_type, is_trimmed
            );
        }

        all_loops_are_vertex_loops
            && self.get_surface_type().unwrap().value
                == PrcCompressedFaceType::PRC_HCG_AnaTorus as u8
            && is_trimmed
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
        let _id = cet.value;
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

    fn VertexColors_get_number_of_colors(&mut self) -> std::io::Result<u32> {
        if self.TESS_3D_Wire_inside {
            return self.t3dw.get_num_vertex_colors();
        }
        Ok(self.VertexColors_number_of_colors)
    }
    /// Number of remaining colors. If the number of colors is 0, this is still 0 and not -1.
    pub fn VertexColors_get_num_colors_remainder(&mut self) -> std::io::Result<u32> {
        let num_vertex_colors = self.VertexColors_get_number_of_colors()?;
        if num_vertex_colors >= 1 {
            Ok(num_vertex_colors - 1)
        } else {
            Ok(0)
        }
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
    pub fn VertexColors_set_num_vertex_colors_from_tess_3d_face(
        &mut self,
        used_entities_flag: u32,
        triangulated_data: &[UnsignedInteger],
    ) -> std::io::Result<()> {
        self.VertexColors_number_of_colors = 0;

        let ue = PrcTessellationBitField::from_bytes(used_entities_flag.to_le_bytes());
        if ue.PRC_FACETESSDATA_Polyface()
            || ue.PRC_FACETESSDATA_PolyfaceOneNormal()
            || ue.PRC_FACETESSDATA_PolyfaceTextured()
            || ue.PRC_FACETESSDATA_PolyfaceOneNormalTextured()
        {
            return Err(std::io::Error::other(
                "PRC_FACETESSDATA_Polyface... not implemented!",
            ));
        }

        use crate::constants::PrcTessellationFlags::*;
        // flag, num_colors_per_triangle, separate_count_first
        let modes = [
            (PRC_FACETESSDATA_Triangle, 3u32, false),
            (PRC_FACETESSDATA_TriangleFan, 1, true),
            (PRC_FACETESSDATA_TriangleStrip, 1, true),
            (PRC_FACETESSDATA_TriangleOneNormal, 3, false),
            (PRC_FACETESSDATA_TriangleFanOneNormal, 1, true),
            (PRC_FACETESSDATA_TriangleStripOneNormal, 1, true),
            (PRC_FACETESSDATA_TriangleTextured, 3, false),
            (PRC_FACETESSDATA_TriangleFanTextured, 1, true),
            (PRC_FACETESSDATA_TriangleStripTextured, 1, true),
            (PRC_FACETESSDATA_TriangleOneNormalTextured, 3, false),
            (PRC_FACETESSDATA_TriangleFanOneNormalTextured, 1, true),
            (PRC_FACETESSDATA_TriangleStripOneNormalTextured, 1, true),
        ];

        let mut i = 0usize; // index into triangulated_data
        for (flag, num_colors_per_triangle, separate_count_first) in modes.iter() {
            if used_entities_flag & (*flag as u32) == 0 {
                continue;
            }
            if i >= triangulated_data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Not enough data to determine number of vertex colors",
                ));
            }
            if !separate_count_first {
                self.VertexColors_number_of_colors +=
                    num_colors_per_triangle * triangulated_data[i].value;
                i += 1;
            } else {
                let num_strips = triangulated_data[i].value;
                i += 1;
                for _j in 0..num_strips {
                    if i >= triangulated_data.len() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Not enough data to determine number of vertex colors",
                        ));
                    }
                    let _is_one_normal =
                        triangulated_data[i].value & PRC_FACETESSDATA_NORMAL_Single as u32 != 0;
                    let tris_in_strip =
                        triangulated_data[i].value & !(PRC_FACETESSDATA_NORMAL_Single as u32);
                    match self
                        .VertexColors_number_of_colors
                        .checked_add(num_colors_per_triangle * tris_in_strip)
                    {
                        Some(val) => self.VertexColors_number_of_colors = val,
                        None => return Err(std::io::Error::other("Too many vertex colors!")),
                    }

                    i += 1;
                }
            }
        }
        Ok(())
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
            let index_str = if index < self.AnaFaceTrimLoop_curves.len() as u32 {
                "valid".to_string()
            } else {
                "invalid".to_string()
            };
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

    pub fn load_prc(
        &mut self,
        bytes: &[u8],
        file_base_name: &str,
        parse: bool,
        opts: &ParsingOptions,
    ) -> std::io::Result<ParsedPrc> {
        debug_time!("load_prc");

        let file_size_bytes = bytes.len();
        debug!("given {} bytes", file_size_bytes);
        let mut mem_reader: Cursor<_> = Cursor::new(bytes);

        let header = UncompressedFileHeader::from_reader(&mut mem_reader, self)?;
        if opts.dump_attachments {
            for (j, file) in header.uncompressed_files.iter().enumerate() {
                let file_name = format!("{}_uncompressed_files_{:03}.bin", file_base_name, j);
                let mut f = std::fs::File::create(file_name)?;
                f.write_all(file.block.a.as_slice())?;
            }
        }

        self.file_base_name = file_base_name.to_owned();
        self.authoring_version = header.authoring_version.value;
        self.num_fsi = header.ufsd.len();
        let mut parsed =
            header.decompress_sections(&mut mem_reader, self, file_size_bytes, opts)?;

        if parse {
            parsed.parse_decompressed(&mut *self, opts)?;
        }

        for (struct_name, info) in &self.struct_stats {
            info!("# of parsed {}: {}", struct_name, info.num);
        }

        Ok(parsed)
    }

    pub fn load_prc_file(
        &mut self,
        infname: &std::string::String,
        parse: bool,
    ) -> std::io::Result<ParsedPrc> {
        let opts: ParsingOptions = ParsingOptions {
            verbose: true,
            all: true,
            globals: true,
            tree: true,
            tess: true,
            geom: true,
            extgeom: true,
            schema: true,
            modelfile: true,
            dump_attachments: false,
        };

        let bytes = std::fs::read(infname)?;
        let file_base_name = Path::new(&infname)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        self.load_prc(&bytes, &file_base_name, parse, &opts)
    }
    pub fn save_prc(
        &mut self,
        outfname: &std::string::String,
        parsed: &ParsedPrc,
    ) -> Result<(), std::io::Error> {
        debug_time!("save_prc");

        let mut bytes: Vec<u8> = vec![];

        UncompressedFileHeader::compress_and_write(&mut bytes, parsed, self)
            .expect("unable to write");

        std::fs::write(outfname, bytes)?;

        Ok(())
    }
    /// Save all tessellated parts into Waveform OBJ.
    pub fn save_obj(
        &mut self,
        outfname: &std::string::String,
        parsed: &ParsedPrc,
    ) -> Result<(), std::io::Error> {
        debug_time!("save_obj(\"{}\")", outfname);

        let base_name = Path::new(&outfname)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
            + "/"
            + Path::new(&outfname).file_stem().unwrap().to_str().unwrap();

        let mut exported = 0u32;
        let mut skipped = 0u32;
        for (fs_id, f) in parsed.fsi.iter().enumerate() {
            for (tess_id, td) in f.tess.tess.iter().enumerate() {
                match &td.id_concrete {
                    PRC_TYPE_TESS_idConcrete::tess_3d(t) => {
                        let file_name = format!("{}-{}-{}.obj", base_name, fs_id, tess_id);
                        info!("writing {}", file_name);
                        let mut w = BufWriter::new(File::create(file_name)?);
                        writeln!(
                            &mut w,
                            "# written by https://github.com/ralovich/prc-rs {}",
                            crate::LIBPRC_VERSION
                        )?;
                        //writeln!(&mut w, "# {}", self.name)?;

                        debug!("{}", t.tessellation_coordinates.coordinates.len());
                        exported += 1;

                        let coordinates = &t.tessellation_coordinates.coordinates;
                        for i in 0..coordinates.len() / 3 {
                            let v = [
                                coordinates[i * 3].value,
                                coordinates[i * 3 + 1].value,
                                coordinates[i * 3 + 2].value,
                            ];
                            writeln!(w, "v {} {} {}", v[0], v[1], v[2])?;
                        }
                        for i in 0..t.normal_coordinates.len() / 3 {
                            let /*mut*/ n = [
                                t.normal_coordinates[i * 3].value,
                                t.normal_coordinates[i * 3 + 1].value,
                                t.normal_coordinates[i * 3 + 2].value,
                            ];
                            //n = Vec3::from(n).normalized().into();
                            writeln!(w, "vn {} {} {}", n[0], n[1], n[2])?;
                        }
                        // for i in 0..t.texture_coordinates.len() / 2 {
                        //     let tc = [
                        //         t.texture_coordinates[i * 2 + 0],
                        //         t.texture_coordinates[i * 2 + 1],
                        //     ];
                        //     writeln!(w, "vt {} {}", tc[0], tc[1])?;
                        // }

                        for face in t.face_tessellation_data.iter() {
                            if face.used_entities_flag.value
                                == PrcTessellationFlags::PRC_FACETESSDATA_Triangle as u32
                            {
                                let num_triangles = face.triangulateddata[0].value;
                                for ti in 0..num_triangles as usize {
                                    let ivert = [
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 1]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 3]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 5]
                                            .value,
                                    ];
                                    let inorm = [
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 2]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 4]
                                            .value,
                                    ];
                                    writeln!(
                                        w,
                                        "f {}//{} {}//{} {}//{}",
                                        ivert[0] / 3 + 1,
                                        inorm[0] / 3 + 1,
                                        ivert[1] / 3 + 1,
                                        inorm[1] / 3 + 1,
                                        ivert[2] / 3 + 1,
                                        inorm[2] / 3 + 1,
                                    )?;
                                }
                            } else if face.used_entities_flag.value
                                == PrcTessellationFlags::PRC_FACETESSDATA_TriangleTextured as u32
                            {
                                let num_triangles = face.triangulateddata[0].value;
                                for ti in 0..num_triangles as usize {
                                    let ivert = [
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 2]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 5]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 8]
                                            .value,
                                    ];
                                    let inorm = [
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 3]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 6]
                                            .value,
                                    ];
                                    let itexc = [
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 1]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 4]
                                            .value,
                                        t.triangulated_index_array
                                            [face.start_triangulated.value as usize + ti * 6 + 7]
                                            .value,
                                    ];
                                    writeln!(
                                        w,
                                        "f {}/{}/{} {}/{}/{} {}/{}/{}",
                                        ivert[0] / 3 + 1,
                                        itexc[0] / 3 + 1,
                                        inorm[0] / 3 + 1,
                                        ivert[1] / 3 + 1,
                                        itexc[1] / 3 + 1,
                                        inorm[1] / 3 + 1,
                                        ivert[2] / 3 + 1,
                                        itexc[2] / 3 + 1,
                                        inorm[2] / 3 + 1,
                                    )?;
                                }
                            } else {
                                warn!(
                                    "unimplemented used_entities_flag={}",
                                    face.used_entities_flag.value
                                );
                            }
                        }
                    }
                    PRC_TYPE_TESS_idConcrete::tess_3d_compressed(t) => {
                        warn!("unimplemented {:?}", PrcType::try_from(t.id.value).unwrap());
                        skipped += 1;
                    }
                    PRC_TYPE_TESS_idConcrete::tess_3d_wire(w) => {
                        warn!("skipping {:?}", PrcType::try_from(w.id.value).unwrap());
                        skipped += 1;
                    }
                    PRC_TYPE_TESS_idConcrete::tess_markup(m) => {
                        warn!("skipping {:?}", PrcType::try_from(m.id.value).unwrap());
                        skipped += 1;
                    }
                    _ => {}
                }
            }
        }
        info!(
            "{}: exported: {}, skipped: {}",
            function!(),
            exported,
            skipped
        );

        Ok(())
    }
}

/// Parse PRC data from a byte array into internal representation.
pub fn prc_describe(
    bytes: &[u8],
    file_base_name: &str,
    opts: &ParsingOptions,
) -> io::Result<ParsedPrc> {
    debug_time!("prc_describe");
    let parse = true;
    let mut ctx: PrcParsingContext = Default::default();
    let parsed = ctx.load_prc(bytes, file_base_name, parse, opts)?;
    Ok(parsed)
}

/// Parse PRC data from a file into internal representation.
pub fn prc_describe_file(fname: &std::string::String, opts: &ParsingOptions) -> io::Result<()> {
    debug_time!("prc_describe_file \"{}\"", fname);

    // Create a path to the desired file
    let path = Path::new(fname);
    let file_base_name = path.file_name().unwrap().to_string_lossy().to_string();
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

    let rv = prc_describe(&bytes, &file_base_name, opts);

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

/// Allow searching for structures in decompressed PRC sections. Helpful for reverse engineering
/// the binary PRC file format.
#[allow(clippy::all)]
pub fn prc_search(
    ctx: &mut PrcParsingContext,
    parsed: &ParsedPrc,
    start_bit: u64,
    end_bit: u64,
) -> Result<(), io::Error> {
    // search decompressed parts bit-by-bit for a value
    let values_to_search_for = [
        /*PrcType::PRC_TYPE_ASM_ModelFile as u32,
        PrcType::PRC_TYPE_TOPO_Context as u32,
        PrcType::PRC_TYPE_ASM_FileStructureGlobals as u32,
        PrcType::PRC_TYPE_ASM_FileStructureTree as u32,
        PrcType::PRC_TYPE_ASM_FileStructureTessellation as u32,
        PrcType::PRC_TYPE_ASM_FileStructureGeometry as u32,
        PrcType::PRC_TYPE_ASM_FileStructureExtraGeometry as u32,
        PrcType::PRC_TYPE_ASM_ProductOccurrence as u32,
        PrcType::PRC_TYPE_ASM_PartDefinition as u32,
        PrcType::PRC_TYPE_MKP_View as u32,*/
        PrcType::PRC_TYPE_TESS_3D as u32,
        PrcType::PRC_TYPE_TESS_3D_Compressed as u32,
        PrcType::PRC_TYPE_TESS_3D_Wire as u32,
        PrcType::PRC_TYPE_TESS_Markup as u32,
    ];
    //for fsi in ctx.prc_parsed.sections_decompressed.iter() {
    for fsi in parsed
        .decompressed_sections
        .as_ref()
        .unwrap()
        .sections_decompressed
        .iter()
    {
        println!(".");
        for (section_id, section) in fsi.iter().enumerate() {
            println!("-");
            if section_id != PrcSectionKind::Tessellation as usize {
                continue;
            }
            //let mut r = std::io::Cursor::new(section.as_slice());
            let endian = bitstream_io::BigEndian;
            let mut r = BitReader::endian(Cursor::new(section.as_slice()), endian);
            let mut bits_consumed = start_bit;
            let mut end_bit = end_bit;
            if end_bit == 0 || end_bit < start_bit {
                end_bit = section.len() as u64 * 8u64;
            }
            info!("End bit is {}", end_bit);
            r.seek_bits(SeekFrom::Start(bits_consumed))?;
            while bits_consumed < end_bit {
                //println!("bp={}", bits_consumed);
                assert_eq!(r.position_in_bits()?, bits_consumed);
                if true {
                    for val in values_to_search_for.iter() {
                        let rv = crate::builtin::UnsignedInteger::from_reader_and_seek_back(&mut r);
                        match rv {
                            Ok(ui) => {
                                if ui.value == *val {
                                    println!(
                                        "{} @ bp={}",
                                        PrcType::try_from(*val).unwrap(),
                                        r.position_in_bits()?
                                    );
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
                            Err(_e) => {}
                        }
                    }
                }
                if false {
                    let rv =
                        crate::builtin::CompressedEntityType::from_reader_and_seek_back(&mut r);
                    match rv {
                        Ok(cet) => {
                            println!("{:?} @ bp={}", cet, r.position_in_bits()?);
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::InvalidData => {}
                        Err(_e) => {}
                    }
                }
                if false {
                    let pos = r.position_in_bits()?;
                    //let rv = BinaryTextureData::from_reader(&mut r, &mut ctx.clone());
                    //let rv = CompressedTextureParameter::from_reader(&mut r, &mut ctx.clone());
                    let rv = PRC_TYPE_TESS_3D_Compressed::from_reader(&mut r, &mut ctx.clone());
                    r.seek_bits(SeekFrom::Start(pos))?;
                    assert_eq!(pos, r.position_in_bits()?);
                    match rv {
                        Ok(btd) => {
                            println!("{:#?} @ bp={}", btd, r.position_in_bits()?);
                        }
                        Err(_e) => {}
                    }
                }
                if false {
                    let pos = r.position_in_bits()?;
                    let rv = crate::builtin::CharacterArray::from_reader_and_seek_back(&mut r, 8);
                    r.seek_bits(SeekFrom::Start(pos))?;
                    match rv {
                        Ok(ca) => {
                            println!("{:?} @ bp={}", ca, r.position_in_bits()?);
                        }
                        Err(_e) => {}
                    }
                }
                if false {
                    let number_of_faces = 65804;
                    let pos = r.position_in_bits()?;
                    let mut ok = true;
                    let rv = ShortArray::from_reader(&mut r, 16);
                    let _ba;
                    if rv.is_ok() {
                        let no_texture = Boolean::from_reader(&mut r);
                        if no_texture.is_ok() {
                            let no_texture = no_texture.unwrap().value;
                            if !no_texture {
                                let td = CompressedTextureParameter::from_reader(
                                    &mut r,
                                    &mut ctx.clone(),
                                );
                                if td.is_ok() {
                                    //println!("FOUND2 @ bp={}", pos);
                                } else {
                                    ok = false;
                                }
                                let all_faces_have_texture = Boolean::from_reader(&mut r);
                                if all_faces_have_texture.is_ok() {
                                    let all_faces_have_texture =
                                        all_faces_have_texture.unwrap().value;
                                    if !all_faces_have_texture {
                                        let face_has_texture = UncompressedBoolArray::from_reader(
                                            &mut r,
                                            number_of_faces,
                                        );
                                        if face_has_texture.is_ok() {
                                        } else {
                                            ok = false;
                                        }
                                    }
                                } else {
                                    ok = false;
                                }
                            } else {
                                //println!("FOUND1 @ bp={}", pos);
                            }
                        } else {
                            ok = false;
                        }
                        let has_behaviors = Boolean::from_reader(&mut r);
                        if has_behaviors.is_ok() {
                            let has_behaviors = has_behaviors.unwrap().value;
                            if has_behaviors {
                                let behaviors_array = CharacterArray::from_reader(&mut r, 8);
                                if behaviors_array.is_ok() {
                                    _ba = behaviors_array.unwrap();
                                } else {
                                    ok = false;
                                }
                            }
                        } else {
                            ok = false;
                        }
                    } else {
                        ok = false;
                    }
                    let after = r.position_in_bits()?;
                    r.seek_bits(SeekFrom::Start(pos))?;
                    if ok
                    /*&& (after > 56528288 - 10000)*/
                    {
                        println!("{:?} @ bp={} after={}", rv?, pos, after);
                    }
                    // match rv {
                    //     Ok(sa) => {
                    //         println!("{:?} @ bp={}", sa, r.position_in_bits()?);
                    //     }
                    //     Err(_e) => {}
                    // }
                }
                read_bits(&mut r, 1)?;
                bits_consumed += 1;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENDIAN: bitstream_io::BigEndian = bitstream_io::BigEndian;

    #[test]
    fn test_describe() {
        let path = std::env::current_dir().unwrap();
        println!(
            "[{}] The current directory is {}",
            function!(),
            path.display()
        );
        let bytes_external = std::fs::read(std::string::String::from(
            "testdata/pmi_sample.stream-23.prc",
        ))
        .unwrap();
        assert_eq!(bytes_external.len(), 24535usize);

        let opts: ParsingOptions = ParsingOptions {
            verbose: true,
            all: true,
            globals: true,
            tree: true,
            tess: true,
            geom: true,
            extgeom: true,
            schema: true,
            modelfile: true,
            dump_attachments: false,
        };

        let parsed = prc_describe(&bytes_external, "pmi_sample.stream-23.prc", &opts);
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
            "[{}] The current directory is {}",
            function!(),
            path.display()
        );
        let bytes_external = std::fs::read(std::string::String::from(
            "testdata/3D-PDF-Sample-School.stream-48.prc",
        ))
        .unwrap();
        assert_eq!(bytes_external.len(), 427862usize);
        let opts: ParsingOptions = ParsingOptions {
            verbose: true,
            all: true,
            globals: true,
            tree: true,
            tess: true,
            geom: true,
            extgeom: true,
            schema: true,
            modelfile: true,
            dump_attachments: false,
        };
        let parsed = prc_describe(&bytes_external, "3D-PDF-Sample-School.stream-48.prc", &opts);
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

    #[test]
    fn test_fuzz1() {
        let file_name = "fuzz.prc".to_owned();
        let _bytes = [10, 68, 68];
        let bytes = [
            80, 82, 67, 201, 31, 0, 0, 201, 31, 0, 0, 67, 65, 68, 69, 34, 241, 52, 255, 246, 1, 0,
            0, 78, 65, 83, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 67, 65,
            68, 69, 34, 241, 52, 255, 248, 1, 0, 0, 78, 65, 83, 0, 0, 0, 0, 0, 6, 0, 0, 0, 107, 0,
            0, 0, 154, 0, 0, 0, 64, 1, 0, 0, 186, 1, 0, 0, 199, 8, 0, 0, 212, 8, 0, 0, 225, 8, 0,
            0, 70, 9, 0, 0, 0, 0, 0, 0, 80, 82, 67, 200, 31, 0, 0, 201, 31, 0, 0, 67, 65, 68, 69,
            34, 241, 52, 255, 248, 1, 0, 0, 78, 65, 83, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 120, 156, 243, 126, 160, 193, 61, 41, 53, 245, 235, 87, 86, 21,
            211, 175, 42, 161, 38, 161, 170, 41, 33, 174, 95, 68, 93, 93, 85, 166, 168, 134, 186,
            132, 2, 133, 77, 166, 61, 119, 11, 87, 123, 38, 118, 52, 45, 92, 221, 108, 218, 183,
            23, 28, 13, 59, 13, 191, 240, 106, 50, 48, 177, 9, 206, 57, 241, 66, 129, 65, 217, 114,
            226, 5, 7, 6, 183, 98, 229, 5, 13, 44, 186, 98, 146, 44, 12, 138, 25, 27, 78, 40, 48,
            114, 187, 182, 57, 50, 112, 36, 49, 8, 28, 97, 80, 236, 96, 16, 16, 251, 197, 146, 96,
            160, 196, 200, 228, 16, 224, 194, 202, 206, 187, 106, 221, 161, 236, 189, 43, 78, 31,
            96, 96, 98, 122, 113, 160, 71, 136, 197, 225, 129, 7, 7, 23, 211, 189, 3, 61, 114, 44,
            142, 11, 28, 20, 116, 88, 255, 51, 32, 0, 0, 78, 148, 51, 187, 120, 156, 155, 225, 224,
            195, 106, 222, 48, 127, 11, 203, 111, 135, 208, 208, 134, 189, 139, 165, 21, 102, 207,
            152, 49, 71, 73, 97, 225, 4, 131, 157, 134, 91, 76, 118, 25, 109, 220, 101, 181, 211,
            104, 130, 228, 172, 25, 250, 51, 156, 19, 92, 140, 88, 22, 104, 112, 112, 229, 50, 74,
            156, 100, 185, 115, 228, 200, 67, 65, 86, 158, 198, 214, 147, 189, 11, 151, 94, 156,
            218, 123, 241, 234, 201, 41, 13, 103, 174, 28, 254, 200, 104, 47, 32, 198, 196, 32,
            185, 144, 65, 176, 185, 252, 57, 131, 128, 3, 195, 116, 7, 39, 190, 134, 6, 6, 56, 0,
            0, 24, 200, 43, 218, 120, 156, 109, 147, 9, 84, 83, 103, 26, 134, 47, 97, 57, 134, 165,
            166, 8, 154, 192, 20, 161, 132, 173, 130, 66, 80, 22, 197, 146, 196, 70, 64, 150, 34,
            4, 12, 120, 80, 139, 104, 145, 176, 28, 33, 154, 136, 165, 13, 8, 52, 96, 168, 1, 146,
            16, 4, 148, 49, 160, 72, 16, 8, 13, 90, 112, 67, 182, 178, 137, 129, 82, 58, 69, 19,
            91, 149, 65, 161, 80, 116, 112, 108, 25, 167, 237, 247, 155, 244, 216, 206, 52, 39,
            231, 185, 247, 61, 247, 255, 182, 247, 126, 87, 214, 21, 102, 220, 228, 103, 234, 205,
            227, 155, 80, 102, 7, 214, 125, 180, 98, 66, 187, 151, 138, 171, 126, 108, 143, 13, 62,
            198, 118, 214, 24, 97, 164, 28, 204, 212, 187, 150, 111, 232, 82, 107, 16, 193, 33, 96,
            7, 150, 48, 215, 219, 15, 48, 226, 176, 194, 254, 210, 162, 99, 244, 177, 173, 152,
            233, 225, 24, 249, 220, 185, 138, 150, 55, 174, 125, 71, 197, 189, 15, 177, 90, 123, 3,
            97, 103, 155, 233, 125, 220, 38, 223, 114, 53, 118, 97, 149, 0, 99, 107, 98, 12, 51,
            248, 167, 119, 215, 47, 97, 110, 125, 121, 152, 249, 25, 190, 49, 177, 149, 106, 80,
            161, 198, 228, 254, 83, 239, 118, 186, 101, 47, 62, 116, 173, 8, 125, 115, 244, 139,
            136, 105, 56, 49, 122, 244, 84, 115, 241, 158, 107, 126, 67, 78, 227, 189, 89, 205,
            171, 58, 80, 3, 18, 93, 9, 74, 171, 174, 196, 116, 242, 242, 227, 250, 185, 30, 97, 19,
            1, 131, 18, 114, 43, 97, 205, 87, 199, 82, 110, 206, 77, 57, 175, 255, 84, 210, 117,
            171, 14, 122, 236, 139, 125, 149, 100, 243, 191, 114, 49, 60, 36, 240, 34, 240, 77,
            110, 240, 141, 160, 176, 95, 243, 216, 243, 177, 208, 111, 62, 221, 19, 194, 81, 254,
            109, 38, 111, 126, 3, 204, 149, 90, 21, 114, 59, 209, 33, 177, 127, 47, 171, 87, 225,
            237, 121, 63, 20, 38, 175, 114, 30, 207, 98, 9, 88, 202, 138, 73, 175, 119, 70, 70,
            124, 199, 151, 49, 171, 196, 91, 152, 5, 151, 255, 42, 9, 73, 77, 53, 36, 46, 218, 27,
            108, 208, 153, 210, 31, 176, 2, 67, 65, 69, 186, 160, 246, 31, 27, 63, 136, 169, 100,
            101, 148, 101, 67, 208, 97, 204, 226, 120, 99, 109, 218, 226, 170, 80, 206, 13, 190,
            177, 127, 3, 213, 144, 212, 133, 67, 22, 135, 248, 148, 159, 238, 165, 247, 181, 228,
            68, 155, 54, 13, 245, 124, 4, 102, 92, 248, 139, 57, 110, 211, 49, 226, 40, 102, 9,
            229, 31, 232, 203, 67, 22, 63, 1, 213, 16, 82, 192, 95, 184, 75, 231, 68, 11, 185, 147,
            245, 89, 214, 173, 163, 40, 139, 159, 46, 203, 67, 15, 235, 53, 219, 134, 190, 219, 7,
            150, 186, 142, 58, 186, 112, 180, 241, 45, 63, 13, 51, 24, 234, 103, 95, 16, 253, 145,
            165, 130, 65, 141, 175, 84, 241, 11, 101, 108, 39, 39, 160, 217, 228, 13, 100, 235, 3,
            10, 55, 124, 186, 241, 87, 161, 138, 153, 176, 114, 93, 238, 9, 100, 173, 181, 238,
            253, 204, 13, 174, 151, 58, 86, 47, 38, 64, 75, 110, 189, 116, 204, 102, 25, 179, 204,
            126, 221, 18, 114, 68, 55, 24, 202, 18, 165, 203, 210, 146, 44, 235, 180, 219, 101,
            104, 130, 90, 178, 12, 175, 251, 167, 146, 160, 100, 223, 103, 118, 43, 40, 173, 247,
            201, 144, 69, 63, 216, 255, 88, 132, 54, 163, 44, 205, 101, 147, 160, 224, 228, 158,
            229, 243, 214, 113, 146, 194, 159, 255, 224, 113, 136, 110, 37, 189, 78, 225, 116, 129,
            175, 29, 33, 181, 233, 130, 72, 57, 251, 45, 155, 70, 122, 114, 92, 254, 255, 173, 66,
            112, 116, 241, 252, 218, 4, 171, 235, 188, 35, 143, 208, 182, 241, 214, 143, 163, 36,
            14, 152, 69, 142, 110, 71, 254, 188, 199, 194, 191, 152, 195, 63, 94, 183, 173, 235,
            140, 48, 39, 100, 107, 172, 126, 83, 111, 189, 74, 144, 85, 163, 223, 210, 34, 88, 173,
            185, 252, 5, 148, 100, 70, 191, 169, 231, 18, 220, 131, 54, 127, 191, 5, 89, 186, 122,
            5, 70, 209, 153, 103, 130, 246, 32, 194, 222, 160, 229, 8, 211, 226, 6, 238, 80, 58,
            250, 92, 124, 5, 216, 254, 245, 77, 121, 87, 186, 74, 194, 209, 199, 240, 231, 61, 54,
            56, 104, 76, 109, 52, 54, 255, 164, 227, 57, 151, 237, 57, 16, 39, 72, 75, 228, 122,
            216, 185, 179, 62, 201, 154, 93, 152, 252, 64, 244, 185, 231, 84, 239, 66, 124, 0, 171,
            245, 151, 153, 175, 53, 77, 61, 14, 7, 29, 218, 139, 238, 169, 120, 202, 253, 255, 189,
            163, 184, 26, 87, 72, 31, 233, 97, 175, 234, 72, 126, 154, 248, 165, 246, 179, 133,
            242, 53, 7, 74, 75, 204, 11, 94, 56, 58, 41, 56, 178, 227, 99, 154, 241, 196, 31, 218,
            188, 190, 23, 221, 120, 148, 212, 158, 94, 190, 50, 123, 75, 103, 88, 130, 116, 251,
            70, 202, 154, 123, 66, 233, 165, 241, 253, 63, 6, 113, 67, 78, 182, 13, 124, 89, 72,
            255, 79, 105, 217, 215, 157, 97, 220, 203, 215, 183, 61, 35, 165, 205, 112, 61, 181,
            83, 207, 87, 14, 28, 184, 82, 163, 237, 57, 206, 158, 8, 116, 144, 106, 185, 89, 111,
            205, 183, 68, 190, 44, 220, 125, 76, 243, 67, 90, 237, 157, 163, 191, 218, 214, 220,
            243, 208, 93, 239, 154, 37, 123, 107, 110, 246, 207, 112, 109, 117, 215, 163, 221, 83,
            230, 47, 50, 158, 118, 104, 126, 191, 42, 47, 159, 216, 122, 247, 101, 58, 247, 176,
            254, 154, 193, 253, 61, 235, 86, 141, 62, 43, 239, 5, 199, 87, 87, 252, 161, 230, 169,
            190, 184, 96, 113, 96, 111, 38, 37, 254, 202, 14, 187, 37, 46, 71, 118, 213, 189, 196,
            134, 100, 182, 176, 61, 52, 246, 73, 144, 126, 80, 249, 54, 135, 156, 129, 193, 20,
            141, 171, 126, 208, 141, 188, 164, 13, 29, 36, 143, 221, 245, 107, 245, 131, 86, 22,
            78, 186, 100, 142, 61, 232, 95, 123, 111, 249, 106, 187, 44, 201, 195, 203, 162, 41,
            98, 90, 222, 39, 212, 27, 29, 159, 104, 203, 170, 25, 14, 58, 73, 212, 27, 173, 26,
            241, 81, 110, 145, 158, 90, 237, 165, 55, 58, 197, 26, 183, 68, 21, 73, 18, 240, 209,
            34, 201, 17, 124, 140, 26, 240, 33, 1, 228, 135, 4, 233, 91, 32, 1, 81, 106, 144, 81,
            106, 105, 17, 146, 69, 248, 48, 244, 20, 144, 139, 14, 231, 226, 147, 144, 76, 34, 136,
            73, 32, 1, 239, 19, 36, 254, 248, 136, 72, 233, 24, 228, 213, 245, 213, 245, 213, 245,
            213, 96, 213, 193, 53, 255, 255, 96, 255, 255, 255, 255, 255, 189, 111, 32, 213, 221,
            213, 213, 247, 42, 213, 213, 149, 80, 65, 3, 7, 7, 7, 7, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 3, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 43, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 37, 0, 0,
            0, 0, 255, 255, 255, 249, 0, 0, 0, 0, 0, 128, 0, 0, 1, 8, 0, 37, 82, 67, 10, 0, 0, 0,
            0, 0, 249, 40, 1, 0, 0, 26, 14, 80, 191, 8, 0, 0, 32, 67, 0, 1, 3, 12, 231, 20, 65,
            238, 247, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 213, 163, 97,
            20, 168, 171, 98, 214, 183, 125, 37, 9, 172, 111, 156, 74, 170, 17, 188, 169, 19, 100,
            11, 19, 170, 90, 108, 65, 71, 40, 9, 142, 122, 175, 189, 62, 52, 170, 174, 128, 140,
            160, 96, 48, 153, 5, 228, 224, 120, 85, 12, 3, 161, 216, 235, 61, 243, 66, 226, 57,
            226, 206, 24, 171, 233, 132, 104, 201, 164, 241, 206, 136, 24, 192, 1, 113, 236, 19,
            184, 91, 29, 199, 72, 151, 0, 24, 209, 146, 18, 70, 186, 152, 25, 227, 207, 154, 0,
            236, 182, 156, 148, 248, 179, 190, 146, 40, 61, 14, 69, 3, 118, 148, 170, 10, 86, 70,
            138, 254, 189, 201, 54, 56, 18, 192, 22, 171, 158, 6, 186, 60, 206, 150, 158, 65, 24,
            108, 156, 237, 148, 158, 113, 75, 61, 221, 172, 184, 8, 184, 230, 110, 214, 221, 213,
            213, 100, 154, 217, 117, 205, 30, 160, 168, 155, 167, 251, 136, 228, 105, 153, 17, 19,
            142, 128, 242, 243, 108, 74, 196, 4, 249, 201, 230, 131, 159, 35, 168, 46, 60, 186,
            125, 48, 108, 82, 252, 178, 242, 146, 219, 178, 93, 174, 60, 217, 102, 54, 87, 17, 108,
            227, 3, 176, 77, 86, 94, 62, 105, 67, 84, 129, 0, 0, 0, 0, 0, 0, 7, 36, 204, 13, 116,
            236, 125, 249, 143, 166, 100, 74, 255, 41, 247, 180, 226, 254, 19, 8, 249, 231, 121,
            88, 103, 42, 250, 224, 178, 212, 235, 8, 141, 174, 193, 221, 169, 215, 207, 15, 157,
            203, 23, 58, 3, 238, 94, 28, 42, 206, 23, 186, 134, 119, 86, 55, 52, 0, 110, 58, 133,
            0, 27, 84, 57, 114, 242, 49, 181, 114, 64, 209, 186, 29, 179, 213, 13, 23, 135, 159,
            169, 154, 17, 58, 220, 204, 55, 156, 140, 190, 83, 116, 90, 212, 211, 8, 168, 115, 41,
            142, 23, 245, 92, 52, 165, 243, 234, 17, 186, 157, 45, 44, 121, 245, 23, 132, 111, 239,
            27, 115, 1, 72, 235, 138, 242, 246, 141, 57, 227, 83, 130, 101, 8, 119, 228, 131, 109,
            228, 75, 111, 135, 54, 6, 84, 33, 140, 200, 205, 112, 193, 50, 178, 112, 217, 179, 21,
            225, 202, 223, 133, 121, 158, 173, 142, 102, 217, 196, 1, 132, 179, 181, 30, 153, 6, 0,
            0, 0, 131, 189, 70, 70, 216, 235, 223, 111, 59, 122, 239, 60, 120, 156, 155, 233, 224,
            193, 0, 7, 0, 13, 234, 1, 34, 120, 156, 155, 121, 192, 131, 1, 1, 0, 21, 140, 1, 162,
            120, 156, 243, 78, 208, 224, 158, 148, 154, 250, 245, 43, 171, 138, 233, 87, 149, 80,
            147, 80, 213, 148, 16, 215, 47, 162, 174, 174, 42, 83, 84, 67, 93, 66, 129, 194, 38,
            211, 158, 187, 133, 171, 61, 19, 59, 154, 22, 174, 110, 54, 237, 219, 11, 142, 134,
            157, 134, 63, 159, 57, 52, 52, 21, 39, 40, 155, 108, 51, 122, 214, 194, 224, 198, 123,
            127, 142, 67, 136, 21, 175, 80, 215, 196, 154, 180, 249, 223, 31, 49, 45, 15, 200, 218,
            224, 98, 64, 0, 0, 129, 232, 34, 248,
        ];
        let opts: ParsingOptions = ParsingOptions {
            verbose: true,
            all: true,
            globals: true,
            tree: true,
            tess: true,
            geom: true,
            extgeom: true,
            schema: true,
            modelfile: true,
            dump_attachments: false,
        };
        let parsed = prc_describe(&bytes, &file_name, &opts);
        assert!(parsed.is_err());
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
            let file_base_name = Path::new(test_case)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let bytes = std::fs::read(test_case).unwrap();
            let mut ctx: PrcParsingContext = Default::default();
            let opts: ParsingOptions = ParsingOptions {
                verbose: true,
                all: true,
                globals: true,
                tree: true,
                tess: true,
                geom: true,
                extgeom: true,
                schema: true,
                modelfile: true,
                dump_attachments: false,
            };
            let parsed = ctx.load_prc(&bytes, &file_base_name, true, &opts).unwrap();
            //println!("{:#?}", prc);

            // serialize and round-trip
            {
                let mut bytes2: Vec<u8> = vec![];

                let _prc2 =
                    UncompressedFileHeader::compress_and_write(&mut bytes2, &parsed, &mut ctx)
                        .unwrap();
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

    #[test]
    fn test_fuzz2() {
        //let data = [255u8, 255, 255, 255, 10];
        //let data = [184, 255, 255, 255, 255u8];
        //let data = [157, 157, 157, 126, 71, 44u8];
        //let data = [131, 1, 0, 1, 47, 128, 147, 4, 17, 130, 255, 251];
        let data = [
            131, 1, 0, 1, 47, 128, 131, 0, 17, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 0, 1, 0, 253, 255, 248, 130, 77, 0, 0,
        ];
        // let data = [
        //     131, 1, 0, 1, 47, 128, 131, 0, 9, 0, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97,
        //     97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 97,
        //     97, 97, 97, 97, 97, 97, 97, 97, 97, 97, 1, 47, 128, 131, 210, 210, 210, 210, 210, 210,
        //     210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 11, 11, 11, 11,
        //     11, 11, 238, 11, 0, 11, 11, 1, 11, 0, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
        //     11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 7, 255, 255, 9, 0, 0, 2, 8, 1, 0,
        //     0, 0, 0, 0, 119, 119, 119, 115, 119, 119, 119, 119, 119, 119, 119, 131, 119, 119,
        // ];
        // let data = [
        //     131, 1, 0, 1, 47, 128, 131, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 126, 210,
        //     122, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 0, 48, 1, 0, 0, 255, 255, 255,
        //     255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 196, 131, 128, 0, 0, 0, 0, 0, 0, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 93, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 1, 0, 1, 47, 128, 131, 0, 131, 255, 0, 0, 0, 0, 0, 0,
        //     128, 131, 0, 17, 0, 255, 255, 255, 173, 0, 131, 131, 1, 0, 1, 47, 128, 131, 0, 17, 0,
        //     0, 17, 0, 1, 0, 0, 0, 0, 0, 85, 255, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 48, 255, 255,
        //     255, 255, 255, 255, 255, 255, 150, 0, 0, 255, 255, 255, 255, 0, 131, 1, 0, 1, 47, 128,
        //     131, 0, 17, 0, 0, 17, 0, 1, 255, 255, 255, 255, 255, 255, 255, 255, 131, 1, 0, 1, 47,
        //     128, 131, 0, 42, 240, 0, 0, 48, 0, 0, 0, 84, 0, 148, 8, 0, 0, 0, 129, 3, 122, 88, 88,
        //     0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        //     255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
        //     0, 220, 0, 0, 0, 0, 0, 1, 0, 1, 237, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 255, 255, 255, 191, 255, 255, 0, 0, 0, 0, 0, 0, 0,
        //     0, 0, 0, 0, 0, 0, 251, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 17,
        //     255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0,
        //     0, 0, 196, 131, 128, 0, 0, 0, 0, 0, 0, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 93, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 1, 0,
        //     1, 47, 128, 131, 0, 131, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127,
        //     127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127, 127,
        //     127, 127, 127, 127, 255, 255, 255, 255, 173, 11, 8, 11, 11, 11, 0, 0, 0, 0, 0, 11, 11,
        //     11,
        // ];
        // let data = [
        //     131, 1, 0, 1, 47, 128, 131, 210, 210, 210, 1, 3, 210, 210, 210, 129, 0, 0, 0, 0, 0, 0,
        //     0, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210, 210,
        //     210, 210, 210, 210, 210, 210, 210, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
        //     11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
        //     11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
        //     11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 210, 210, 210, 0, 0, 0,
        //     21, 0, 0, 0, 0, 0,
        // ];

        let mut ctx = PrcParsingContext::default();

        let mut r = BitReader::endian(Cursor::new(&data), ENDIAN);
        let _schema = Schema::from_reader(&mut r, &mut ctx);
        let _reference = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx);
        // if let Ok(schema) = Schema::from_reader(&mut r, &mut ctx) {
        //     if let Ok(reference) = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r, &mut ctx)
        //     {
        //         let mut bytes: Vec<u8> = vec![];
        //         let mut w = bitstream_io::BitWriter::endian(Cursor::new(&mut bytes), ENDIAN);
        //         schema
        //             .to_writer(&mut w, &mut ctx)
        //             .expect("Write should succeed!");
        //         reference
        //             .to_writer(&mut w, &mut ctx)
        //             .expect("Write should succeed!");
        //         crate::test_common::fill_partial_byte_at_end(&mut w, false)
        //             .expect("failed to fill partial byte at end");
        //
        //         let mut r = BitReader::endian(Cursor::new(&bytes), ENDIAN);
        //         let rec_schema = crate::prc_gen::Schema::from_reader(&mut r, &mut ctx)
        //             .expect("Re-parsing should work!");
        //         let recovered = crate::prc_gen::PRC_TYPE_ASM_FileStructureGlobals::from_reader(
        //             &mut r, &mut ctx,
        //         )
        //         .expect("Re-parsing should work!");
        //         assert_eq!(schema, rec_schema);
        //         assert_eq!(reference, recovered);
        //     }
        // }

        // let file_name = "fuzz_target_2.prc".to_owned();
        // // fuzzed code goes here
        //
        // let mut ctx = PrcParsingContext::default();
        // let bytes = ctx
        //     .mutate_section(PrcSectionKind::Global, data.as_slice())
        //     .unwrap();
        //
        // let _ = prc_describe(
        //     bytes.as_slice(),
        //     &file_name,
        //     true,
        //     true,
        //     true,
        //     true,
        //     true,
        //     true,
        //     true,
        //     true,
        //     true,
        // );
    }
}
