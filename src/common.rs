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
//use crate::vec3::Vec3;
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
        for i in 0..self.uncompr_files.len() {
            num_bytes += self.uncompr_files[i].len() as u32;
        }
        num_bytes
    }
    pub fn parse_decompressed2(
        &mut self,
        ctx: &mut PrcParsingContext,
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
        if self.decompressed_sections.is_none() {
            return Ok(());
        }
        let parsed_sections = UncompressedFileHeader::parse_decompressed_sections2(
            &self.decompressed_sections.as_ref().unwrap(),
            ctx,
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
        self.fsi = parsed_sections.fsi;
        self.mf_schema = parsed_sections.mf_schema;
        self.mf = parsed_sections.mf;
        self.decompressed_sections = None;

        Ok(())
    }
}

/// Accumulates state of parsing.
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
        triangulated_data: &Vec<UnsignedInteger>,
    ) {
        // TODO

        let num_colors_per_triangle;
        if used_entities_flag != PrcTessellationFlags::PRC_FACETESSDATA_Triangle as u32
            && used_entities_flag != PrcTessellationFlags::PRC_FACETESSDATA_TriangleTextured as u32
        {
            warn!(
                "Only PRC_FACETESSDATA_Triangle and PRC_FACETESSDATA_TriangleTextured are implemented! VertexColors_number_of_colors will be off!"
            );
        }
        num_colors_per_triangle = 3;

        self.VertexColors_number_of_colors = 0;
        for i in 0..triangulated_data.len() {
            self.VertexColors_number_of_colors +=
                num_colors_per_triangle * triangulated_data[i].value;
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

    pub fn load_prc00(
        &mut self,
        bytes: &[u8],
        file_base_name: &std::string::String,
        parse: bool,
        verbose: bool,
        all: bool,
        globals: bool,
        tree: bool,
        tess: bool,
        geom: bool,
        extgeom: bool,
        _schema: bool,
        modelfile: bool,
    ) -> std::io::Result<ParsedPrc> {
        debug_time!("load_prc");

        let file_size_bytes = bytes.len();
        debug!("given {} bytes", file_size_bytes);
        let mut mem_reader: Cursor<_> = Cursor::new(bytes);

        let header = UncompressedFileHeader::from_reader(&mut mem_reader, self)?;

        self.file_base_name = file_base_name.clone();
        self.authoring_version = header.authoring_version.value;
        self.num_fsi = header.ufsd.len();
        let mut parsed = header.decompress_sections(
            &mut mem_reader,
            self,
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

        if parse {
            parsed.parse_decompressed2(
                &mut *self, verbose, all, globals, tree, tess, geom, extgeom, _schema, modelfile,
            )?;
        }

        Ok(parsed)
    }

    pub fn load_prc(
        &mut self,
        infname: &std::string::String,
        parse: bool,
    ) -> std::io::Result<ParsedPrc> {
        let verbose = true;
        let all = true;
        let globals = true;
        let tree = true;
        let tess = true;
        let geom = true;
        let extgeom = true;
        let _schema = true;
        let modelfile = true;

        let bytes = std::fs::read(infname)?;
        let file_base_name = Path::new(&infname)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        self.load_prc00(
            &bytes,
            &file_base_name,
            parse,
            verbose,
            all,
            globals,
            tree,
            tess,
            geom,
            extgeom,
            _schema,
            modelfile,
        )
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
                                coordinates[i * 3 + 0].value,
                                coordinates[i * 3 + 1].value,
                                coordinates[i * 3 + 2].value,
                            ];
                            writeln!(w, "v {} {} {}", v[0], v[1], v[2])?;
                        }
                        for i in 0..t.normal_coordinates.len() / 3 {
                            let /*mut*/ n = [
                                t.normal_coordinates[i * 3 + 0].value,
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
                                            [face.start_triangulated.value as usize + ti * 6 + 0]
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
                                            [face.start_triangulated.value as usize + ti * 6 + 0]
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

    /// Helper for generating PRC sections from a stream of bytes.
    /// Used for fuzzing.
    #[allow(unused)]
    pub fn mutate_section(
        &mut self,
        section: PrcSectionKind,
        data: &[u8],
    ) -> std::io::Result<Vec<u8>> {
        if section == PrcSectionKind::Global {
            let mut parsed = ParsedPrc::default();
            parsed.verread = crate::LIBPRC_PRC_SPEC_VERSION;
            parsed.verauth = crate::LIBPRC_PRC_SPEC_VERSION;
            parsed.fsi.push(ParsedPrcFileStructure::default());
            parsed.fsi[0].header.magic.a = b"PRC".to_vec();
            parsed.fsi[0].header.minimal_version_for_read.value = crate::LIBPRC_PRC_SPEC_VERSION;
            parsed.fsi[0].header.authoring_version.value = crate::LIBPRC_PRC_SPEC_VERSION;

            let mut bytes: Vec<u8> = vec![];
            UncompressedFileHeader::compress_and_write_override_globals(
                &mut bytes, &parsed, self, data,
            )?;
            return Ok(bytes);
        }
        return Ok(vec![]);
    }
}

pub fn prc_describe(
    bytes: &[u8],
    file_base_name: &std::string::String,
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
    let parse = true;
    let mut ctx: PrcParsingContext = Default::default();
    let parsed = ctx.load_prc00(
        bytes,
        file_base_name,
        parse,
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
    Ok(parsed)
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

    let rv = prc_describe(
        &bytes,
        &file_base_name,
        verbose,
        all,
        globals,
        tree,
        tess,
        geom,
        extgeom,
        _schema,
        modelfile,
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

/// Allow searching for structures in decompressed PRC sections. Helpful for reverse engineering
/// the binary PRC file format.
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
        for section_id in 0..fsi.len() {
            println!("-");
            let section = &fsi[section_id];
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
                if false {
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
                if true {
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
        let bytes_external = std::fs::read(&std::string::String::from(
            "testdata/pmi_sample.stream-23.prc",
        ))
        .unwrap();
        assert_eq!(bytes_external.len(), 24535usize);

        let parsed = prc_describe(
            &bytes_external,
            &"pmi_sample.stream-23.prc".to_owned(),
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
            "[{}] The current directory is {}",
            function!(),
            path.display()
        );
        let bytes_external = std::fs::read(&std::string::String::from(
            "testdata/3D-PDF-Sample-School.stream-48.prc",
        ))
        .unwrap();
        assert_eq!(bytes_external.len(), 427862usize);

        let parsed = prc_describe(
            &bytes_external,
            &"3D-PDF-Sample-School.stream-48.prc".to_owned(),
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

    #[test]
    fn test_fuzz() {
        let file_name = "fuzz.prc".to_owned();
        let bytes = [10, 68, 68];
        let parsed = prc_describe(
            &bytes, &file_name, true, true, true, true, true, true, true, true, true,
        );
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
            let parsed = ctx
                .load_prc00(
                    &bytes,
                    &file_base_name,
                    true,
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
