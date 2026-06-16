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
use bitstream_io::BitReader;
use log::{debug, trace};
use std::io;

impl AnaFaceTrimLoop {
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
}
