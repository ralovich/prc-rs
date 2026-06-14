// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(unused)]

/// Built-in structures that are byte-aligned.
use crate::common::{ParsedPrc, ParsedPrcFileStructure, PrcParsingContext};
use crate::constants::PrcSectionKind;
use crate::decompress;
use crate::decompress::decompress;
use crate::function;
use crate::prc_gen::{
    PRC_TYPE_ASM_FileStructureExtraGeometry, PRC_TYPE_ASM_FileStructureGeometry,
    PRC_TYPE_ASM_FileStructureGlobals, PRC_TYPE_ASM_FileStructureTessellation,
    PRC_TYPE_ASM_FileStructureTree, UncompressedBlock, UncompressedFileHeader,
    UncompressedFileStructureDescription, UncompressedFileStructureHeader, UncompressedUniqueId,
};
use bitstream_io::{BitReader, BitWrite, BitWriter};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, info, trace, warn};
use measure_time::debug_time;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Seek, Write};
use std::{fmt, io};

impl UncompressedUniqueId {
    pub fn from(uuid: [u32; 4]) -> UncompressedUniqueId {
        UncompressedUniqueId {
            unique_id: [
                UncompressedUnsignedInteger { value: uuid[0] },
                UncompressedUnsignedInteger { value: uuid[1] },
                UncompressedUnsignedInteger { value: uuid[2] },
                UncompressedUnsignedInteger { value: uuid[3] },
            ],
        }
    }
}

impl UncompressedFileHeader {
    /// Once the header is loaded from the binary PRC file, the second step is to read structures
    /// that are located at offsets. Third step is to decompress these sections. Fourth is to parse
    /// them.
    /// Results are stored in ctx.prc_parsed.
    pub fn decompress_sections<R: Read + Seek>(
        &self,
        rdr: &mut R,
        ctx: &mut PrcParsingContext,
        file_size_bytes: usize,
        verbose: bool,
        all: bool,
        _globals: bool,
        _tree: bool,
        _tess: bool,
        _geom: bool,
        _extgeom: bool,
        _schema: bool,
        _modelfile: bool,
    ) -> std::io::Result<()> {
        ctx.ver_for_reading = self.minimal_version_for_read.value;
        ctx.ver_authoring = self.authoring_version.value;
        ctx.prc_parsed.verread = self.minimal_version_for_read.value;
        ctx.prc_parsed.verauth = self.authoring_version.value;

        info!("Version for reading: {}", ctx.ver_for_reading);
        info!("Authoring version: {}", ctx.ver_authoring);
        info!("Number of file sections: {}", self.num_file_structs.value);
        info!("num_uncompr_files: {}", self.num_uncompr_files.value);

        // decompress arrays
        let mf_size = self.mf_end_offset.value - self.mf_start_offset.value;
        // trace!(
        //     "mf compressed offset: [{},{}], size: {}",
        //     self.mf_start_offset.value, self.mf_end_offset.value, mf_size
        // );
        let mut mf_compr: Vec<u8> = vec![0; mf_size as usize];
        rdr.seek(std::io::SeekFrom::Start(self.mf_start_offset.value as u64))?;
        rdr.read_exact(&mut mf_compr)?;

        let mut sections_decompressed: Vec<[Vec<u8>; PrcSectionKind::Count as usize]> =
            vec![Default::default(); self.fsi.len()];
        for i in 0..self.fsi.len() {
            let fs = &self.fsi[i];

            let section_size = fs.glob_start_offset.value - fs.header_start_offset.value;
            let mut section: Vec<u8> = vec![0; section_size as usize];
            rdr.seek(std::io::SeekFrom::Start(
                fs.header_start_offset.value as u64,
            ))?;
            rdr.read_exact(&mut section)?;
            sections_decompressed[i][PrcSectionKind::Header as usize] = section;

            let section_size = fs.tree_start_offset.value - fs.glob_start_offset.value;
            let mut section_compr: Vec<u8> = vec![0; section_size as usize];
            rdr.seek(std::io::SeekFrom::Start(fs.glob_start_offset.value as u64))?;
            rdr.read_exact(&mut section_compr)?;
            let glob = decompress(&section_compr).unwrap();
            sections_decompressed[i][PrcSectionKind::Global as usize] = glob;

            if all || _tree {
                let section_size = fs.tess_start_offset.value - fs.tree_start_offset.value;
                let mut section_compr: Vec<u8> = vec![0; section_size as usize];
                rdr.seek(std::io::SeekFrom::Start(fs.tree_start_offset.value as u64))?;
                rdr.read_exact(&mut section_compr)?;
                let tree = decompress(&section_compr).unwrap();
                sections_decompressed[i][PrcSectionKind::Tree as usize] = tree;
            }

            if all || _tess {
                let section_size = fs.geom_start_offset.value - fs.tess_start_offset.value;
                let mut section_compr: Vec<u8> = vec![0; section_size as usize];
                rdr.seek(std::io::SeekFrom::Start(fs.tess_start_offset.value as u64))?;
                rdr.read_exact(&mut section_compr)?;
                let tess = decompress(&section_compr).unwrap();
                sections_decompressed[i][PrcSectionKind::Tessellation as usize] = tess;
            }

            if all || _geom {
                let section_size = fs.extg_start_offset.value - fs.geom_start_offset.value;
                let mut section_compr: Vec<u8> = vec![0; section_size as usize];
                rdr.seek(std::io::SeekFrom::Start(fs.geom_start_offset.value as u64))?;
                rdr.read_exact(&mut section_compr)?;
                let geom = decompress(&section_compr).unwrap();
                sections_decompressed[i][PrcSectionKind::Geometry as usize] = geom;
            }

            if all || _extgeom {
                let section_size =
                    std::cmp::min(self.mf_start_offset.value, file_size_bytes as u32)
                        - fs.extg_start_offset.value;
                let mut section_compr: Vec<u8> = vec![0; section_size as usize];
                rdr.seek(std::io::SeekFrom::Start(fs.extg_start_offset.value as u64))?;
                rdr.read_exact(&mut section_compr)?;
                let extg = decompress(&section_compr).unwrap();
                sections_decompressed[i][PrcSectionKind::ExtraGeometry as usize] = extg;
            }
        }

        // parse decompressed arrays
        ctx.prc_parsed.uuid_file = *self
            .unique_id_file
            .unique_id
            .iter()
            .map(|f| f.value)
            .collect::<Vec<u32>>()
            .as_array()
            .unwrap();
        ctx.prc_parsed.uuid_application = *self
            .unique_id_application
            .unique_id
            .iter()
            .map(|f| f.value)
            .collect::<Vec<u32>>()
            .as_array()
            .unwrap();
        ctx.prc_parsed.uncompr_files = self
            .uncompressed_files
            .clone()
            .into_iter()
            .map(|f| f.block.a)
            .collect();

        let endian = bitstream_io::BigEndian;
        let mut sum_files = 0;
        for i in 0..self.fsi.len() {
            let fs = &self.fsi[i];

            let mut r_head =
                Cursor::new(sections_decompressed[i][PrcSectionKind::Header as usize].as_slice());
            let head = UncompressedFileStructureHeader::from_reader(&mut r_head, ctx)?;
            sum_files += head.files.len();
        }
        info!("sum files: {}", sum_files);
        for i in 0..self.fsi.len() {
            debug!("--fsi[{}] len={} START--", i, self.fsi.len());
            let fs = &self.fsi[i];

            let mut r_head =
                Cursor::new(sections_decompressed[i][PrcSectionKind::Header as usize].as_slice());
            let head = UncompressedFileStructureHeader::from_reader(&mut r_head, ctx)?;

            let mut r_glob = BitReader::endian(
                Cursor::new(sections_decompressed[i][PrcSectionKind::Global as usize].as_slice()),
                endian,
            );
            let schema = crate::prc_gen::Schema::from_reader(&mut r_glob, ctx)?;
            ctx.se = crate::schema::SchemaEvaluator::new(&schema.schemas);
            let mut glob = Default::default();
            if all || _globals {
                let id = PrcSectionKind::Global;
                let bytes = sections_decompressed[i][id as usize].as_slice();
                debug!(
                    "--{:?} START {} bits ({} bytes)--",
                    id,
                    bytes.len() * 8,
                    bytes.len()
                );
                let now = std::time::Instant::now();

                glob = PRC_TYPE_ASM_FileStructureGlobals::from_reader(&mut r_glob, ctx)?;

                if verbose {
                    debug_time!("{:?} verbose", id);
                    let _str = format!("{:#?}", &glob);
                    debug!("{}", _str);
                }
                let total_bits = (bytes.len() * 8) as u64;
                let consumed_bits = r_glob.position_in_bits()?;
                let remaining_bits = total_bits - consumed_bits;
                debug!(
                    "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                    id,
                    remaining_bits,
                    remaining_bits as f64 / total_bits as f64 * 100.0,
                    consumed_bits,
                    consumed_bits as f64 / total_bits as f64 * 100.0,
                    total_bits,
                    bytes.len(),
                    now.elapsed().as_millis()
                );
                if remaining_bits > 7 {
                    warn!("--{:?} many uninterpreted tailing bits! --", id);
                }
            }

            let mut tree = Default::default();
            if all || _tree {
                let id = PrcSectionKind::Tree;
                let bytes = sections_decompressed[i][id as usize].as_slice();
                debug!(
                    "--{:?} START {} bits ({} bytes)--",
                    id,
                    bytes.len() * 8,
                    bytes.len()
                );
                let now = std::time::Instant::now();

                let mut r_tree = BitReader::endian(Cursor::new(bytes), endian);
                tree = PRC_TYPE_ASM_FileStructureTree::from_reader(&mut r_tree, ctx)?;
                if verbose {
                    debug_time!("{:?} verbose", id);
                    let _str = format!("{:#?}", &tree);
                    debug!("{}", _str);
                }
                let total_bits = (bytes.len() * 8) as u64;
                let consumed_bits = r_tree.position_in_bits()?;
                let remaining_bits = total_bits - consumed_bits;
                debug!(
                    "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                    id,
                    remaining_bits,
                    remaining_bits as f64 / total_bits as f64 * 100.0,
                    consumed_bits,
                    consumed_bits as f64 / total_bits as f64 * 100.0,
                    total_bits,
                    bytes.len(),
                    now.elapsed().as_millis()
                );
                if remaining_bits > 7 {
                    warn!("--{:?} many uninterpreted tailing bits! --", id);
                }
            }

            let mut tess = Default::default();
            if all || _tess {
                let id = PrcSectionKind::Tessellation;
                let bytes = sections_decompressed[i][id as usize].as_slice();
                debug!(
                    "--{:?} START {} bits ({} bytes)--",
                    id,
                    bytes.len() * 8,
                    bytes.len()
                );
                let now = std::time::Instant::now();

                let mut r_tess = BitReader::endian(
                    Cursor::new(sections_decompressed[i][id as usize].as_slice()),
                    endian,
                );
                tess = PRC_TYPE_ASM_FileStructureTessellation::from_reader(&mut r_tess, ctx)?;

                if verbose {
                    debug_time!("{:?} verbose", id);
                    let _str = format!("{:#?}", &tess);
                    debug!("{}", _str);
                }
                let total_bits = (bytes.len() * 8) as u64;
                let consumed_bits = r_tess.position_in_bits()?;
                let remaining_bits = total_bits - consumed_bits;
                debug!(
                    "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                    id,
                    remaining_bits,
                    remaining_bits as f64 / total_bits as f64 * 100.0,
                    consumed_bits,
                    consumed_bits as f64 / total_bits as f64 * 100.0,
                    total_bits,
                    bytes.len(),
                    now.elapsed().as_millis()
                );
                if remaining_bits > 7 {
                    warn!("--{:?} many uninterpreted tailing bits! --", id);
                }
            }

            let mut geom = Default::default();
            if all || _geom {
                let id = PrcSectionKind::Geometry;
                let bytes = sections_decompressed[i][id as usize].as_slice();
                debug!(
                    "--{:?} START {} bits ({} bytes)--",
                    id,
                    bytes.len() * 8,
                    bytes.len()
                );
                let now = std::time::Instant::now();

                let mut r_geom = BitReader::endian(
                    Cursor::new(sections_decompressed[i][id as usize].as_slice()),
                    endian,
                );
                geom = PRC_TYPE_ASM_FileStructureGeometry::from_reader(&mut r_geom, ctx)?;

                if verbose {
                    debug_time!("{:?} verbose", id);
                    let _str = format!("{:#?}", &geom);
                    debug!("{}", _str);
                }
                let total_bits = (bytes.len() * 8) as u64;
                let consumed_bits = r_geom.position_in_bits()?;
                let remaining_bits = total_bits - consumed_bits;
                debug!(
                    "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                    id,
                    remaining_bits,
                    remaining_bits as f64 / total_bits as f64 * 100.0,
                    consumed_bits,
                    consumed_bits as f64 / total_bits as f64 * 100.0,
                    total_bits,
                    bytes.len(),
                    now.elapsed().as_millis()
                );
                if remaining_bits > 7 {
                    warn!("--{:?} many uninterpreted tailing bits! --", id);
                }
            }

            let mut extg = Default::default();
            if all || _extgeom {
                let id = PrcSectionKind::ExtraGeometry;
                let bytes = sections_decompressed[i][id as usize].as_slice();
                debug!(
                    "--{:?} START {} bits ({} bytes)--",
                    id,
                    bytes.len() * 8,
                    bytes.len()
                );
                let now = std::time::Instant::now();

                let mut r_extg = BitReader::endian(
                    Cursor::new(sections_decompressed[i][id as usize].as_slice()),
                    endian,
                );
                extg = PRC_TYPE_ASM_FileStructureExtraGeometry::from_reader(&mut r_extg, ctx)?;

                if verbose {
                    debug_time!("{:?} verbose", id);
                    let _str = format!("{:#?}", &extg);
                    debug!("{}", _str);
                }
                let total_bits = (bytes.len() * 8) as u64;
                let consumed_bits = r_extg.position_in_bits()?;
                let remaining_bits = total_bits - consumed_bits;
                debug!(
                    "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                    id,
                    remaining_bits,
                    remaining_bits as f64 / total_bits as f64 * 100.0,
                    consumed_bits,
                    consumed_bits as f64 / total_bits as f64 * 100.0,
                    total_bits,
                    bytes.len(),
                    now.elapsed().as_millis()
                );
                if remaining_bits > 7 {
                    warn!("--{:?} many uninterpreted tailing bits! --", id);
                }
            }

            ctx.prc_parsed.fsi.push(ParsedPrcFileStructure {
                uuid: [
                    fs.unique_id.unique_id[0].value,
                    fs.unique_id.unique_id[1].value,
                    fs.unique_id.unique_id[2].value,
                    fs.unique_id.unique_id[3].value,
                ],
                header: head,
                schema,
                glob,
                tree,
                tess,
                geom,
                extg,
            });
            debug!("--fsi[{}] len={} ENDOK--", i, self.fsi.len());
        }
        let mut mf_decompressed: Vec<u8> = Vec::new();
        if all || _modelfile {
            mf_decompressed = decompress(&mf_compr).unwrap();
            trace!(
                "mf uncompressed {} -> {}",
                mf_compr.len(),
                mf_decompressed.len()
            );

            let bytes = mf_decompressed.as_slice();
            debug!(
                "--{:?} START {} bits ({} bytes)--",
                "ModelFile",
                bytes.len() * 8,
                bytes.len()
            );
            let now = std::time::Instant::now();

            let mut r_mf = BitReader::endian(Cursor::new(bytes), endian);
            ctx.prc_parsed.mf_schema = crate::prc_gen::Schema::from_reader(&mut r_mf, ctx)?;
            ctx.prc_parsed.mf =
                crate::prc_gen::PRC_TYPE_ASM_ModelFile::from_reader(&mut r_mf, ctx)?;

            if verbose {
                debug_time!("{:?} verbose", "ModelFile");
                let _str = format!("{:#?}", &ctx.prc_parsed.mf);
                debug!("{}", _str);
            }
            let total_bits = (bytes.len() * 8) as u64;
            let consumed_bits = r_mf.position_in_bits()?;
            let remaining_bits = total_bits - consumed_bits;
            debug!(
                "--{:?} ENDOK remaining: {} bits ({:.2}%), consumed: {} bits ({:.2}%) of {} bits ({} bytes) [took {} ms]--",
                "ModelFile",
                remaining_bits,
                remaining_bits as f64 / total_bits as f64 * 100.0,
                consumed_bits,
                consumed_bits as f64 / total_bits as f64 * 100.0,
                total_bits,
                bytes.len(),
                now.elapsed().as_millis()
            );
            if remaining_bits > 7 {
                warn!("--{:?} many uninterpreted tailing bits! --", "ModelFile");
            }
        }
        ctx.prc_parsed.sections_decompressed = sections_decompressed;
        ctx.prc_parsed.mf_decompressed = mf_decompressed;

        Ok(())
    }

    /// ParsedPrc -> UncompressedFileHeader
    pub fn compress_and_write<W: Write>(
        w: &mut W,
        ctx: &mut PrcParsingContext,
    ) -> std::io::Result<Self> {
        let parsed_prc = &ctx.prc_parsed;
        let mut w_ctx = ctx.clone();

        let endian = bitstream_io::BigEndian;

        let mut mf_compressed = vec![];
        {
            let mut mf_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut mf_uncompressed), endian);
            parsed_prc.mf_schema.to_writer(&mut w, &mut w_ctx)?;
            parsed_prc.mf.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            mf_compressed = decompress::compress(mf_uncompressed.as_slice()).unwrap();
        }

        let mut fsi: Vec<UncompressedFileStructureDescription> =
            vec![Default::default(); parsed_prc.fsi.len()];
        let mut sections_compressed: Vec<[Vec<u8>; 6]> =
            vec![Default::default(); parsed_prc.fsi.len()];

        let mut mf_start_offset = 0;

        for i in 0..parsed_prc.fsi.len() {
            let fs = &parsed_prc.fsi[i];

            fs.header
                // UncompressedFileStructureHeader {
                //     magic: UncompressedByteArray { a: b"PRC".to_vec() },
                //     minimal_version_for_read: UncompressedUnsignedInteger {
                //         value: fs.header.verread,
                //     },
                //     authoring_version: UncompressedUnsignedInteger {
                //         value: fs.header.verauth,
                //     },
                //     unique_id_file: UncompressedUniqueId::from(fs.header.uuid_file),
                //     unique_id_application: UncompressedUniqueId::from(fs.header.uuid_application),
                //     file_count: UncompressedUnsignedInteger {
                //         value: fs.header.files.len() as u32,
                //     },
                //     files: fs
                //         .header
                //         .files
                //         .clone()
                //         .into_iter()
                //         .map(|f| UncompressedBlock {
                //             block_size: UncompressedUnsignedInteger {
                //                 value: f.len() as u32,
                //             },
                //             block: UncompressedByteArray { a: f },
                //         })
                //         .collect(),
                // }
                .to_writer(
                    &mut sections_compressed[i][PrcSectionKind::Header as usize],
                    &mut w_ctx,
                )?;
            assert_eq!(
                47,
                sections_compressed[i][PrcSectionKind::Header as usize].len()
            );

            let mut section_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut section_uncompressed), endian);
            fs.schema.to_writer(&mut w, &mut w_ctx)?;
            fs.glob.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            sections_compressed[i][PrcSectionKind::Global as usize] =
                decompress::compress(section_uncompressed.as_slice()).unwrap();

            let mut section_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut section_uncompressed), endian);
            fs.tree.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            sections_compressed[i][PrcSectionKind::Tree as usize] =
                decompress::compress(section_uncompressed.as_slice()).unwrap();

            let mut section_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut section_uncompressed), endian);
            fs.tess.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            sections_compressed[i][PrcSectionKind::Tessellation as usize] =
                decompress::compress(section_uncompressed.as_slice()).unwrap();

            let mut section_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut section_uncompressed), endian);
            fs.geom.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            sections_compressed[i][PrcSectionKind::Geometry as usize] =
                decompress::compress(section_uncompressed.as_slice()).unwrap();

            let mut section_uncompressed = vec![];
            let mut w = BitWriter::endian(Cursor::new(&mut section_uncompressed), endian);
            fs.extg.to_writer(&mut w, &mut w_ctx)?;
            w.byte_align()?;
            sections_compressed[i][PrcSectionKind::ExtraGeometry as usize] =
                decompress::compress(section_uncompressed.as_slice()).unwrap();

            let header_start_offset = 47
                + (i as u32 + 1) * (48 + fs.header.files_size())
                + 12
                + parsed_prc.uncompr_files_size();
            let glob_start_offset = header_start_offset + 47;
            let tree_start_offset = glob_start_offset + sections_compressed[i][1].len() as u32;
            let tess_start_offset = tree_start_offset + sections_compressed[i][2].len() as u32;
            let geom_start_offset = tess_start_offset + sections_compressed[i][3].len() as u32;
            let extg_start_offset = geom_start_offset + sections_compressed[i][4].len() as u32;
            mf_start_offset = extg_start_offset + sections_compressed[i][5].len() as u32;

            fsi[i] = UncompressedFileStructureDescription {
                unique_id: UncompressedUniqueId::from(fs.uuid),
                reserved: UncompressedUnsignedInteger { value: 0 },
                section_count: UncompressedUnsignedInteger { value: 6 },
                header_start_offset: UncompressedUnsignedInteger {
                    value: header_start_offset,
                },
                glob_start_offset: UncompressedUnsignedInteger {
                    value: glob_start_offset,
                },
                tree_start_offset: UncompressedUnsignedInteger {
                    value: tree_start_offset,
                },
                tess_start_offset: UncompressedUnsignedInteger {
                    value: tess_start_offset,
                },
                geom_start_offset: UncompressedUnsignedInteger {
                    value: geom_start_offset,
                },
                extg_start_offset: UncompressedUnsignedInteger {
                    value: extg_start_offset,
                },
            };
        }

        let file_header = UncompressedFileHeader {
            magic: UncompressedByteArray { a: b"PRC".to_vec() },
            minimal_version_for_read: UncompressedUnsignedInteger {
                value: parsed_prc.verread,
            },
            authoring_version: UncompressedUnsignedInteger {
                value: parsed_prc.verauth,
            },
            unique_id_file: UncompressedUniqueId::from(parsed_prc.uuid_file),
            unique_id_application: UncompressedUniqueId::from(parsed_prc.uuid_application),
            num_file_structs: UncompressedUnsignedInteger {
                value: fsi.len() as u32,
            },
            fsi,
            mf_start_offset: UncompressedUnsignedInteger {
                value: mf_start_offset,
            },
            mf_end_offset: UncompressedUnsignedInteger {
                value: mf_start_offset + mf_compressed.len() as u32,
            },
            num_uncompr_files: UncompressedUnsignedInteger {
                value: 0, /*FIXME*/
            },
            uncompressed_files: Default::default(),
        };

        file_header.to_writer(w, &mut w_ctx)?;
        for i in 0..sections_compressed.len() {
            for j in 0..6 {
                w.write(sections_compressed[i][j].as_slice())?;
            }
        }
        w.write(mf_compressed.as_slice())?;

        Ok(file_header)
    }
}

impl UncompressedFileStructureHeader {
    pub fn files_size(&self) -> u32 {
        let mut num_bytes = 0;
        for i in 0..self.files.len() {
            num_bytes += self.files[i].block.a.len() as u32;
        }
        num_bytes
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UncompressedByteArray {
    pub a: Vec<u8>,
}
impl UncompressedByteArray {
    pub fn from_reader<R: Read>(rdr: &mut R, num_bytes: u32) -> io::Result<Self> {
        let mut bytes = vec![0; num_bytes as usize];
        rdr.read_exact(&mut bytes)?;
        Ok(Self { a: bytes })
        // Ok(Self {
        //     a: rdr.read_to_vec(num_bytes as usize)?,
        // })
    }
    pub fn to_writer<W: Write + ?Sized>(&self, w: &mut W, _: u32) -> std::io::Result<()> {
        //w.write_bytes(&self.a)
        w.write_all(&self.a)
    }
}
impl fmt::Debug for UncompressedByteArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "UncompressedByteArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "UncompressedByteArray {} elements", self.a.len()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq, Copy)]
pub struct UncompressedUnsignedInteger {
    pub value: u32,
}
impl UncompressedUnsignedInteger {
    pub fn new() -> Self {
        UncompressedUnsignedInteger { value: 0 }
    }
    pub fn from_reader<R: Read>(rdr: &mut R) -> io::Result<Self> {
        let mut bytes: [u8; 4] = [0; 4];
        let _ = rdr.read_exact(&mut bytes)?;
        let mut ui: u32 = bytes[0] as u32;
        ui |= (bytes[1] as u32) << 8;
        ui |= (bytes[2] as u32) << 16;
        ui |= (bytes[3] as u32) << 24;
        Ok(Self { value: ui })
    }
    pub fn to_writer<W: Write + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let mut val = self.value;
        let mut bytes: [u8; 4] = [0; 4];
        bytes[0] = (val & 0xFF) as u8;
        val >>= 8;
        bytes[1] = (val & 0xFF) as u8;
        val >>= 8;
        bytes[2] = (val & 0xFF) as u8;
        val >>= 8;
        bytes[3] = (val & 0xFF) as u8;
        for i in 0..4 {
            //let _ = w.write::<8, _>(bytes[i])?;
            let _ = w.write_u8(bytes[i]);
        }
        Ok(())
    }
}
