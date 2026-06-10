// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(unreachable_code)]
#![allow(unused)]

use crate::constants::*;
use crate::prc_gen::*;
use crate::{double, indent};
use bitstream_io::{BitRead, BitReader, BitWrite};
use byteorder::{LittleEndian, ReadBytesExt};
use measure_time::debug_time;
use num_enum::TryFromPrimitive;
//use std::convert::TryFrom;
use crate::common::PrcParsingContext;
use crate::constants;
use crate::constants::PrcCompressedFaceType::PRC_HCG_NewLoop;
use crate::decompress::decompress;
use crate::function;
//use crate::prc_builtin::CompressedEntityTypeKind::{ComprCurv, ComprFace};
use crate::builtin::{Boolean, CompressedEntityType, UnsignedIntegerWithVariableBitNumber};
use crate::prc_gen::{AnaFaceTrimLoop, CompressedMultiplicitiesU, CompressedMultiplicitiesV};
use log::{debug, info, trace, warn};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::io::{/*Cursor,*/ Read, Seek, SeekFrom};

impl AnaFaceTrimLoop {
    pub fn from_reader_while_loop2<
        R: std::io::Read + std::io::Seek,
        E: bitstream_io::Endianness,
    >(
        rdr: &mut BitReader<R, E>,
        _ctx: &mut PrcParsingContext,
    ) -> io::Result<Vec<Self>> {
        trace!(
            "{}AnaFaceTrimLoop::from_reader_while_loop() bp={}",
            indent::get(),
            rdr.position_in_bits()?
        );
        let _ig = indent::IndentGuard::new();
        let mut loop_surface_orientation: Boolean = Default::default();
        let mut curve_type: u8 = PrcCompressedFaceType::PRC_HCG_NewLoop as u8;

        // TODO: compute: number_of_loop, number_of_curves
        let mut loops = vec![];

        // TODO: is this enough, or it should be curve_type_tmp.is_PRC_HCG_NewLoop()?
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
                //let element = RefOrCompressedCurve::from_reader(rdr, _ctx)?;
                // open coding RefOrCompressedCurve::from_reader()...
                let mut curve = RefOrCompressedCurve::default();
                curve.curve_is_not_already_stored = Boolean::from_reader(rdr)?;
                if curve.curve_is_not_already_stored.value {
                    let curve_type_tmp = CompressedEntityType::from_reader_and_seek_back(rdr)?;
                    //curve_type = CompressedEntityType::from_reader_and_seek_back(rdr)?.value;
                    trace!("{}{:?}", indent::get(), curve_type_tmp);
                    //if (curve_type == PrcCompressedFaceType::PRC_HCG_NewLoop as u32) || (curve_type == PrcCompressedFaceType::PRC_HCG_EndLoop as u32) {
                    if curve_type_tmp.is_PRC_HCG_NewLoop() || curve_type_tmp.is_PRC_HCG_EndLoop() {
                        curve_type = CompressedEntityType::from_reader(rdr)?.value;
                        break;
                    }
                    // readCompressedCurveOfType(curveType);
                    curve.compressed_curve = Some(CompressedCurve::from_reader(rdr, _ctx)?);
                } else {
                    // readCurveRef();
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
