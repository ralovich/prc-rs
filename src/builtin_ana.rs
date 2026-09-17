// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use crate::builtin::{Boolean, CompressedEntityType, UnsignedIntegerWithVariableBitNumber};
use crate::common::PrcParsingContext;
use crate::constants::*;
use crate::indent;
use crate::prc_gen::AnaFaceTrimLoop;
use crate::prc_gen::*;
use bitstream_io::{BitReader, BitWrite};
use log::{debug, trace};
use std::io;

impl AnaFaceTrimLoop {
    /// https://github.com/pdf-association/pdf-issues/issues/696
    pub fn from_reader_while_loop<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
        _ctx: &mut PrcParsingContext,
    ) -> io::Result<Vec<Self>> {
        trace!(
            "{}AnaFaceTrimLoop::from_reader_while_loop() bp={}",
            indent::get(),
            rdr.position_in_bits()?
        );
        let _ig = indent::IndentGuard::new();
        let mut loop_surface_orientation: Boolean;
        let mut curve_type: u8 = PrcCompressedFaceType::PRC_HCG_NewLoop as u8;

        let mut loops = vec![];

        while curve_type == PrcCompressedFaceType::PRC_HCG_NewLoop as u8 {
            _ctx.AnaFaceTrimLoop_start_new_loop();
            let mut curves: Vec<RefOrCompressedCurve> = Vec::new();

            loop_surface_orientation = Boolean::from_reader(rdr)?;
            debug!(
                "{}loop_surface_orientation: {:?}",
                indent::get(),
                loop_surface_orientation.value
            );
            loop {
                // open coding RefOrCompressedCurve::from_reader()...
                let mut curve = RefOrCompressedCurve::default();
                curve.curve_is_not_already_stored = Boolean::from_reader(rdr)?;
                if curve.curve_is_not_already_stored.value {
                    let curve_type_tmp = CompressedEntityType::from_reader_and_seek_back(rdr)?;
                    trace!("{}{:?}", indent::get(), curve_type_tmp);
                    if curve_type_tmp.is_PRC_HCG_NewLoop() || curve_type_tmp.is_PRC_HCG_EndLoop() {
                        curve_type = CompressedEntityType::from_reader(rdr)?.value;
                        break;
                    }
                    curve.compressed_curve = Some(CompressedCurve::from_reader(rdr, _ctx)?);
                } else {
                    curve.index_compressed_curve =
                        Some(UnsignedIntegerWithVariableBitNumber::from_reader(
                            rdr,
                            _ctx.BrepDataCompress_number_of_bits_to_store_reference,
                        )?);
                }
                _ctx.AnaFaceTrimLoop_add_curve_to_loop(curve.clone());
                curves.push(curve);
            }
            _ctx.AnaFaceTrimLoop_store_loop();
            loops.push(Self {
                loop_surface_orientation,
                curves,
            });
        }
        Ok(loops)
    }
    pub fn to_writer_loops<W: BitWrite + ?Sized>(
        w: &mut W,
        ctx: &mut PrcParsingContext,
        loops: &[Self],
    ) -> std::io::Result<()> {
        for loop_id in 0..loops.len() {
            let _loop = &loops[loop_id];
            _loop.loop_surface_orientation.to_writer(w)?;

            for _curve in _loop.curves.iter() {
                _curve.to_writer(w, ctx)?;
            }

            if loop_id == loops.len() - 1 {
                Boolean { value: true }.to_writer(w)?;
                CompressedEntityType {
                    value: PrcCompressedFaceType::PRC_HCG_EndLoop as u8,
                    is_a_curve: false,
                }
                .to_writer(w)?;
            } else {
                Boolean { value: true }.to_writer(w)?;
                CompressedEntityType {
                    value: PrcCompressedFaceType::PRC_HCG_NewLoop as u8,
                    is_a_curve: false,
                }
                .to_writer(w)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::CompressedPoint;
    use crate::test_common::*;
    use bitstream_io::BitWriter;
    use std::io::Cursor;

    #[test]
    fn io_content_compressed_ana_face() {
        let mut bytes = vec![];
        let mut ctx = PrcParsingContext::default();
        ctx.push_face_type(CompressedEntityType {
            value: crate::constants::PrcCompressedFaceType::PRC_HCG_IsoPlane as u8,
            is_a_curve: false,
        });
        ctx.set_curve_trimming_face(false);
        ctx.brep_data_compressed_tolerance = 0.001;
        let ccaf = ContentCompressedAnaFace {
            is_trimmed: Boolean { value: true },
            trim_loop: Some(vec![
                AnaFaceTrimLoop {
                    loop_surface_orientation: Boolean { value: true },
                    curves: vec![
                        RefOrCompressedCurve {
                            curve_is_not_already_stored: Boolean { value: true },
                            index_compressed_curve: None,
                            compressed_curve: Some(CompressedCurve {
                                id_concrete: CompressedCurve_idConcrete::line(PRC_HCG_Line {
                                    id: CompressedEntityType::new_curve(
                                        PrcCompressedCurveType::PRC_HCG_Line,
                                    ),
                                    start_end_data: StartEndData {
                                        start_vertex: None,
                                        end_vertex: None,
                                        start_point: Some(CompressedPoint {
                                            x: 1.0,
                                            y: 2.0,
                                            z: -4.0,
                                        }),
                                        end_point: Some(CompressedPoint {
                                            x: 2.0,
                                            y: 4.0,
                                            z: 8.0,
                                        }),
                                    },
                                }),
                            }),
                        },
                        RefOrCompressedCurve {
                            curve_is_not_already_stored: Boolean { value: true },
                            index_compressed_curve: None,
                            compressed_curve: Some(CompressedCurve {
                                id_concrete: CompressedCurve_idConcrete::circ(PRC_HCG_Circle {
                                    id: Some(CompressedEntityType::new_curve(
                                        PrcCompressedCurveType::PRC_HCG_Circle,
                                    )),
                                    is_particular_circle: Boolean { value: false },
                                    particular_circle: None,
                                    general_circle: Some(GeneralCircle {
                                        start_end_data: Some(StartEndData {
                                            start_vertex: None,
                                            end_vertex: None,
                                            start_point: Some(CompressedPoint {
                                                x: -1.0,
                                                y: -2.0,
                                                z: 4.0,
                                            }),
                                            end_point: Some(CompressedPoint {
                                                x: 8.0,
                                                y: -4.0,
                                                z: 2.0,
                                            }),
                                        }),
                                        center: CompressedPoint {
                                            x: 1.0,
                                            y: 4.0,
                                            z: -2.0,
                                        },
                                        circle_angle: Boolean { value: true },
                                    }),
                                }),
                            }),
                        },
                    ],
                },
                AnaFaceTrimLoop {
                    loop_surface_orientation: Boolean { value: true },
                    curves: vec![],
                },
            ]),
            point_on_torus: None,
        };

        let endian = bitstream_io::LittleEndian;
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), endian);
            ccaf.to_writer(&mut w, &mut ctx).unwrap();
            fill_partial_byte_at_end(&mut w, true).unwrap();
        }

        let bytes_ro = bytes.as_slice();
        let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
        let recovered = ContentCompressedAnaFace::from_reader(&mut r, &mut ctx).unwrap();
        assert_eq!(ccaf, recovered);
    }
}
