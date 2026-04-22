// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(unreachable_code)]
#![allow(unused)]

use crate::constants::*;
use crate::prc_double;
use bitstream_io::{BitRead, BitReader, BitWrite};
use byteorder::{LittleEndian, ReadBytesExt};
use measure_time::debug_time;
use num_enum::TryFromPrimitive;
//use std::convert::TryFrom;
use crate::constants;
use crate::decompress::decompress;
use crate::function;
use crate::prc_builtin::CompressedEntityTypeKind::{ComprCurv, ComprFace};
use crate::prc_gen::{CompressedMultiplicitiesU, CompressedMultiplicitiesV};
use crate::indent;
use log::{debug, info, trace, warn};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::io::{/*Cursor,*/ Read, Seek, SeekFrom};

pub fn have_bbox(bounding_box_behavior: i8) -> bool {
    let bf = PrcBodyBoundingBoxBehaviorBitField::from_bytes([bounding_box_behavior as u8]);
    bf.PRC_BODY_BBOX_Evaluation() || bf.PRC_BODY_BBOX_Precise()
}

/// https://github.com/pdf-association/pdf-issues/issues/705#issuecomment-3680110465
pub fn is_an_iso_face(type_id: u32) -> bool {
    use PrcCompressedFaceType::*;
    match type_id.try_into() {
        Ok(PRC_HCG_IsoPlane) => true,
        Ok(PRC_HCG_IsoCylinder) => true,
        Ok(PRC_HCG_IsoTorus) => true,
        Ok(PRC_HCG_IsoSphere) => true,
        Ok(PRC_HCG_IsoCone) => true,
        Ok(PRC_HCG_IsoNurbs) => true,
        Ok(PRC_HCG_NewLoop)
        | Ok(PRC_HCG_EndLoop)
        | Ok(PRC_HCG_AnaPlane)
        | Ok(PRC_HCG_AnaCylinder)
        | Ok(PRC_HCG_AnaTorus)
        | Ok(PRC_HCG_AnaSphere)
        | Ok(PRC_HCG_AnaCone)
        | Ok(PRC_HCG_AnaNurbs)
        | Ok(PRC_HCG_AnaGenericFace) => false,
        _ => panic!("Cannot tell if ISO face? (type_id: {})", type_id),
    }
    // match type_id {
    //     #[allow(non_upper_case_globals)]
    //     ((PRC_HCG_IsoPlane as u32)..(PRC_HCG_IsoNurbs as u32)) | PrcCompressedFaceType::PRC_HCG_IsoCylinder | PrcCompressedFaceType::PRC_HCG_IsoTorus | PrcCompressedFaceType::PRC_HCG_IsoSphere | PRC_HCG_IsoCone | PRC_HCG_IsoNurbs => true,
    //     #[allow(non_upper_case_globals)]
    //     PRC_HCG_AnaPlane | PRC_HCG_AnaCylinder | PRC_HCG_AnaTorus | PRC_HCG_AnaSphere | PRC_HCG_AnaCone | PRC_HCG_AnaNurbs | PRC_HCG_AnaGenericFace => false,
    //     _ => panic!("Cannot tell if ISO face?")
    // }
}

fn max(i: i32, j: i32, k: i32) -> i32 {
    std::cmp::max(std::cmp::max(i, j), k)
}

/// See ContentCompressedFace in spec.
///
/// Vertex loops are used to represent a loop consisting of a single
/// vertex, such as might exist on the apex of a cone, or a sphere
/// touching a plane. They are represented by a degenerate line which
/// has identical start and end vertices.
pub fn all_loops_are_vertex_loops() -> bool {
    // TODO
    warn!("FIXME: all_loops_are_vertex_loops: not implemented yet");

    // from https://github.com/pdf-association/pdf-issues/issues/696#issuecomment-3599474152
    // Which means that ALL the loops in this array of loops are degenerate lines (a line with two
    // points that are the same with precision/tolerance), the surface type is PRC_HCG_AnaTorus and
    // is_trimmed is TRUE.

    // if (start-end).len() < tol {
    //     return true;
    // }
    return false;
}

/// Print info about a Vec<T> without printing all elements.
pub fn format<T: std::cmp::Ord + std::fmt::Display>(v: &Vec<T>) -> std::string::String {
    let min_value = v.iter().min();
    let max_value = v.iter().max();
    match (min_value, max_value) {
        (Some(min), Some(max)) => format!(
            "Vec<{}> {} elements, range: [{}, {}]",
            std::any::type_name::<T>(),
            v.len(),
            min,
            max
        ),
        (_, _) => format!("Vec<{}> {} elements", std::any::type_name::<T>(), v.len()),
    }
}

pub fn sum_up_u(mult: &Vec<CompressedMultiplicitiesU>) -> u32 {
    pub fn get_multiplicity(mult: &Vec<CompressedMultiplicitiesU>, i: usize) -> u32 {
        if i == 0 {
            assert!(!mult[i].multiplicity_is_stored);
            return mult[i].multiplicity.unwrap().value;
        } else {
            if !mult[i].multiplicity_is_stored {
                return mult[i].multiplicity.unwrap().value;
            } else {
                return get_multiplicity(mult, i - 1);
            }
        }
    }
    let mut accum = 0_u32;
    for i in 0..mult.len() {
        accum += get_multiplicity(mult, i);
    }
    return accum;
}

pub fn sum_up_v(mult: &Vec<CompressedMultiplicitiesV>) -> u32 {
    pub fn get_multiplicity(mult: &Vec<CompressedMultiplicitiesV>, i: usize) -> u32 {
        if i == 0 {
            assert!(!mult[i].multiplicity_is_stored);
            return mult[i].multiplicity.unwrap().value;
        } else {
            if !mult[i].multiplicity_is_stored {
                return mult[i].multiplicity.unwrap().value;
            } else {
                return get_multiplicity(mult, i - 1);
            }
        }
    }
    let mut accum = 0_u32;
    for i in 0..mult.len() {
        accum += get_multiplicity(mult, i);
    }
    return accum;
}

#[derive(Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Boolean {
    pub value: bool,
}
impl Boolean {
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let value: bool = rdr.read_bit().unwrap();
        Ok(Boolean { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_bit(self.value)
    }
}
impl std::ops::Not for Boolean {
    type Output = bool;
    fn not(self) -> bool {
        match self.value {
            true => false,
            false => true,
        }
    }
}
impl fmt::Debug for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl Serialize for Boolean {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_bool(self.value)
    }
}
impl<'de> Deserialize<'de> for Boolean {
    fn deserialize<D>(deserializer: D) -> Result<Boolean, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct BoolVisitor;

        impl<'de> serde::de::Visitor<'de> for BoolVisitor {
            type Value = Boolean;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a boolean value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Boolean, E>
            where
                E: serde::de::Error,
            {
                Ok(crate::prc_builtin::Boolean { value })
            }
        }
        deserializer.deserialize_bool(BoolVisitor)
    }
}


#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    pub value: i8,
}
impl Character {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let value = rdr.read_to::<i8>().unwrap();
        Ok(Character { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        w.write::<8, _>(self.value)
    }
}
impl fmt::Debug for Character {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedCharacter {
    pub value: u8,
}
impl UnsignedCharacter {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let value: u8 = rdr.read_to::<u8>().unwrap();
        Ok(UnsignedCharacter { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        w.write::<8, _>(self.value)
    }
}
impl fmt::Debug for UnsignedCharacter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedShort {
    pub value: u16,
}
impl UnsignedShort {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let lo = rdr.read_to::<u8>().unwrap();
        let hi = rdr.read_to::<u8>().unwrap();
        let value: u16 = (hi as u16) << 8 | lo as u16;
        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let lo = (self.value & 0xFF) as u8;
        let hi = (self.value >> 8) as u8;
        w.write::<8, _>(lo)?;
        w.write::<8, _>(hi)
    }
}
impl fmt::Debug for UnsignedShort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedInteger {
    pub value: u32,
}
impl UnsignedInteger {
    pub fn new() -> Self {
        UnsignedInteger { value: 0 }
    }
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let mut ui: u32 = 0;
        let mut i: u32 = 0;
        while rdr.read_bit()? && i < 4 {
            let ux8: u8 = rdr.read_to::<u8>()?;
            let ux: u32 = ux8 as u32;
            let sh: u32 = 8 * i;
            ui |= ux << sh;
            i = i + 1;
        }
        Ok(Self { value: ui })
    }
    pub fn from_reader_and_seek_back<
        R: std::io::Read + std::io::Seek,
        E: bitstream_io::Endianness,
    >(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let pos = rdr.position_in_bits().unwrap();
        let value: u32 = Self::from_reader(rdr)?.value;
        rdr.seek_bits(SeekFrom::Start(pos))?;
        assert_eq!(pos, rdr.position_in_bits().unwrap());
        Ok(Self { value })
    }
    pub fn search_and_seek_back<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
        needle: u32,
        //start_offset_bits: i64,
        max_offset_bits: u64,
        max_found_count: u32,
    ) -> Vec<u64> {
        let pos = rdr.position_in_bits().unwrap();

        let needle_str;
        match PrcType::try_from(needle) {
            Ok(val) => needle_str = val.to_string(),
            Err(_) => needle_str = needle.to_string(),
        }

        info!(
            "[Starting searching for value:{}, starting bit pos:{}]",
            needle_str, pos
        );
        let mut found_count = 0;
        let mut offsets = Vec::with_capacity(max_found_count as usize);
        for offset in 0_u64..max_offset_bits {
            rdr.seek_bits(SeekFrom::Start(pos + offset)).unwrap();
            let read_rv = Self::from_reader(rdr);
            let value: u32;
            match read_rv {
                Ok(val) => {
                    value = val.value;
                }
                Err(err) => {
                    value = needle + 1;
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        rdr.seek_bits(SeekFrom::Start(pos)).unwrap();
                        return offsets;
                    }
                }
            }
            rdr.seek_bits(SeekFrom::Start(pos)).unwrap();
            if value == needle {
                info!(
                    "[Search found value:{} at bit abs:{} offset:{}]",
                    needle_str,
                    pos + offset,
                    offset
                );
                found_count += 1;
                offsets.push(offset);
            }
            if found_count > max_found_count {
                break;
            }
        }
        offsets
    }
    pub fn search_and_read<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
        needle: u32,
        //start_offset_bits: i64,
        //max_offset_bits: u64,
        //max_found_count: u32,
    ) -> io::Result<Self> {
        let max_offset_bits: u64 = 25;
        let max_found_count: u32 = 1;
        let max_allowed_offset = 10;
        let found_offsets = Self::search_and_seek_back(rdr, needle, max_offset_bits, max_found_count);
        dbg!(&found_offsets);
        if !found_offsets.is_empty() {
            if found_offsets[0] <= max_allowed_offset {
                rdr.seek_bits(SeekFrom::Current(found_offsets[0] as i64))?;
                let ui = Self::from_reader(rdr)?;
                return Ok(ui);
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("[search_and_read did not find {} within {} bits]", needle, max_allowed_offset)))
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let mut val = self.value;
        loop {
            if val == 0 {
                return w.write_bit(false);
            }
            w.write_bit(true)?;
            let uc: u8 = (val & 0xFF) as u8;
            w.write::<8, _>(uc)?;
            val = val >> 8;
        }
    }
}
impl fmt::Debug for UnsignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl Serialize for UnsignedInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_u32(self.value)
    }
}
impl<'de> Deserialize<'de> for UnsignedInteger {
    fn deserialize<D>(deserializer: D) -> Result<UnsignedInteger, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct U32Visitor;

        impl<'de> serde::de::Visitor<'de> for U32Visitor {
            type Value = UnsignedInteger;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer between 0 and 2^32")
            }

            fn visit_u32<E>(self, value: u32) -> Result<UnsignedInteger, E>
            where
                E: serde::de::Error,
            {
                Ok(crate::prc_builtin::UnsignedInteger { value })
            }
        }
        deserializer.deserialize_u32(U32Visitor)
    }
}


#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct UncompressedUnsignedInteger {
    pub value: u32,
}
impl UncompressedUnsignedInteger {
    pub fn new() -> Self {
        UncompressedUnsignedInteger { value: 0 }
    }
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let mut bytes: [u8; 4] = [0; 4];
        let _ = rdr.read_bytes(&mut bytes)?;
        let mut ui: u32 = bytes[0] as u32;
        ui |= (bytes[1] as u32) << 8;
        ui |= (bytes[2] as u32) << 16;
        ui |= (bytes[3] as u32) << 24;
        Ok(UncompressedUnsignedInteger { value: ui })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
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
            let _ = w.write::<8, _>(bytes[i])?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct String {
    pub value: std::string::String,
}
impl String {
    pub fn new() -> Self {
        String {
            value: std::string::String::new(),
        }
    }
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let is_not_empty: bool = Boolean::from_reader(rdr)?.value;
        let mut value: std::string::String = std::string::String::new();
        if is_not_empty {
            let str_len: u32 = UnsignedInteger::from_reader(rdr)?.value;
            for _i in 0..str_len {
                let uc8: u8 = rdr.read_to().unwrap();
                let uc: char = uc8 as char;
                value.push(uc);
            }
        }
        Ok(String { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        if self.value.is_empty() {
            return w.write_bit(false);
        }
        w.write_bit(true)?;
        let bytes = self.value.clone().into_bytes();
        let size = UnsignedInteger {
            value: bytes.len() as u32,
        };
        size.to_writer(w)?;
        let mut ui = 0;
        while ui < bytes.len() {
            let c = bytes[ui];
            let uc = UnsignedCharacter { value: c };
            uc.to_writer(w)?;
            ui = ui + 1;
        }
        Ok(())
    }
}
impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct Integer {
    pub value: i32,
}
impl Integer {
    pub fn new() -> Self {
        Integer { value: 0 }
    }
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let mut ii: i32 = 0;
        let mut j: i32 = 0;
        while Boolean::from_reader(rdr)?.value {
            let ival8: u8 = rdr.read_to::<u8>().unwrap();
            let ival: i32 = ival8 as i32;
            ii |= ival << 8 * j;
            j += 1;
        }
        if j > 0 {
            ii <<= (4 - j) * 8;
            ii >>= (4 - j) * 8;
        }
        Ok(Integer { value: ii })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let mut val = self.value;
        if val == 0 {
            return w.write_bit(false);
        }
        loop {
            let loc = val & 0xFF;
            w.write_bit(true)?;
            let uc: u8 = (val & 0xFF) as u8;
            w.write::<8, _>(uc)?;

            val = val >> 8;
            if (val == 0 && (loc & 0x80) == 0) || (val == -1 && (loc & 0x80) != 0) {
                return w.write_bit(false);
            }
        }
    }
}
impl fmt::Debug for Integer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Default, Clone, Copy, PartialOrd)]
pub struct Double {
    pub value: f64,
}
impl Double {
    pub fn new() -> Self {
        Double { value: 0.0 }
    }
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let d = prc_double::read_double_from_reader(rdr)?;
        Ok(Double { value: d })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        prc_double::write_double_to_writer(w, self.value)
    }
}
impl PartialEq for Double {
    fn eq(&self, other: &Self) -> bool {
        self.value.total_cmp(&other.value) == std::cmp::Ordering::Equal
    }
}
impl Eq for Double {}
impl Ord for Double {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.total_cmp(&other.value)
    }
}
impl fmt::Debug for Double {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl fmt::Display for Double {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
impl Serialize for Double {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_f64(self.value)
    }
}
impl<'de> Deserialize<'de> for Double {
    fn deserialize<D>(deserializer: D) -> Result<Double, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct F64Visitor;

        impl<'de> serde::de::Visitor<'de> for F64Visitor {
            type Value = Double;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("f64 value")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Double, E>
            where
                E: serde::de::Error,
            {
                Ok(Double { value: value as f64 })
            }

            fn visit_i64<E>(self, value: i64) -> Result<Double, E>
            where
                E: serde::de::Error,
            {
                Ok(Double { value: value as f64 })
            }

            fn visit_f64<E>(self, value: f64) -> Result<Double, E>
            where
                E: serde::de::Error,
            {
                Ok(Double { value })
            }
        }
        deserializer.deserialize_f64(F64Visitor)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UserData {
    #[serde(skip)]
    pub data: Vec<bool>, // FIXME consider BitVec?
}
impl UserData {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        trace!("{}UserData::from_reader()", indent::get());
        let num_bits: u32 = UnsignedInteger::from_reader(rdr)?.value;
        let mut data: Vec<bool> = Vec::with_capacity(num_bits as usize);
        for _i in 0..num_bits {
            data.push(rdr.read_bit()?);
        }
        Ok(UserData { data })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        UnsignedInteger {
            value: self.data.len() as u32,
        }
        .to_writer(w)?;
        for v in &self.data {
            Boolean { value: *v }.to_writer(w)?;
        }
        Ok(())
    }
}
impl fmt::Debug for UserData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} bits", self.data.len())
    }
}

/// Getnumberofbitsusedtostoreunsignedinteger in spec
fn get_number_of_bits_used_to_store_unsigned_integer(u: u32) -> u32 {
    let mut nb = 2;
    let mut tmp = 2;
    while u >= tmp {
        tmp *= 2;
        nb += 1;
    }
    nb - 1
}
/// GetNumberOfBitsUsedToStoreInteger() in the spec
fn get_number_of_bits_used_to_store_integer(i: i32) -> u32 {
    let u = i.abs() as u32;
    return get_number_of_bits_used_to_store_unsigned_integer(u) + 1;
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedIntegerWithVariableBitNumber {
    //pub num_bits: u8, // shall be less than 31
    pub value: u32,
}
impl UnsignedIntegerWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(rdr: &mut R, num_bits: u32) -> io::Result<Self> {
        //println!("UnsignedIntegerWithVariableBitNumber: {}", num_bits);
        assert!(num_bits > 0);
        assert!(num_bits < 31);
        let mut value = 0u32;
        for u in 0..num_bits {
            let b: u32 = ((rdr.read_bit()? as u8) & 0x01) as u32;
            value |= b << (num_bits - u - 1);
        }
        Ok(UnsignedIntegerWithVariableBitNumber { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, num_bits: u32) -> std::io::Result<()> {
        assert!(num_bits > 0);
        assert!(num_bits < 31);
        let mut uval = self.value;
        for u in 0..num_bits {
            let test = 1 << (num_bits - 1 - u);
            if uval >= test {
                let _ = w.write_bit(true)?;
                uval -= test;
            } else {
                let _ = w.write_bit(false)?;
            }
        }
        Ok(())
    }
}
impl fmt::Debug for UnsignedIntegerWithVariableBitNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
struct IntegerWithVariableBitNumber {
    pub value: i32,
}
impl IntegerWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(r: &mut R, num_bits: u32) -> io::Result<Self> {
        assert!(num_bits > 1);
        assert!(num_bits < 31);

        let is_neg = r.read_bit()?;
        let ui = UnsignedIntegerWithVariableBitNumber::from_reader(r, num_bits - 1)?.value;
        let value = if !is_neg { ui as i32 } else { -(ui as i32) };

        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, num_bits: u32) -> std::io::Result<()> {
        assert!(num_bits > 1);
        assert!(num_bits < 31);

        w.write_bit(self.value < 0)?;
        UnsignedIntegerWithVariableBitNumber {
            value: self.value.abs() as u32,
        }
        .to_writer(w, num_bits - 1)?;
        Ok(())
    }
}
impl fmt::Debug for IntegerWithVariableBitNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct NumberOfBitsThenUnsignedInteger {
    pub value: u32,
}
impl NumberOfBitsThenUnsignedInteger {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        let num_bits = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 5)?.value;
        let value: u32 = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, num_bits)?.value;
        Ok(NumberOfBitsThenUnsignedInteger { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let num_bits = get_number_of_bits_used_to_store_unsigned_integer(self.value);
        UnsignedIntegerWithVariableBitNumber { value: num_bits }.to_writer(w, 5)?;
        UnsignedIntegerWithVariableBitNumber { value: self.value }.to_writer(w, num_bits)?;
        Ok(())
    }
}
impl fmt::Debug for NumberOfBitsThenUnsignedInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq /*, TryFromPrimitive*/)]
#[allow(non_camel_case_types)]
//#[repr(u32)]
pub enum CompressedEntityTypeKind {
    Invalid(u32),
    ComprCurv(PrcCompressedCurveType),
    ComprFace(PrcCompressedFaceType),
}
impl Default for CompressedEntityTypeKind {
    fn default() -> Self {
        CompressedEntityTypeKind::Invalid(u32::MAX)
    }
}
impl fmt::Display for CompressedEntityTypeKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedEntityType {
    pub value: u32,
    pub is_a_curve: bool,
    pub aid: CompressedEntityTypeKind, // NOT serialized
}
impl CompressedEntityType {
    pub fn from_reader_and_seek_back<
        R: std::io::Read + std::io::Seek,
        E: bitstream_io::Endianness,
    >(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let pos = rdr.position_in_bits().unwrap();
        let rv = CompressedEntityType::from_reader(rdr)?;
        rdr.seek_bits(SeekFrom::Start(pos))?;
        assert_eq!(pos, rdr.position_in_bits().unwrap());
        Ok(rv)
    }
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        trace!("{}CompressedEntityType::from_reader()", indent::get());
        let is_a_curve = rdr.read_bit()?;
        let typev: u32;
        let e: CompressedEntityTypeKind;
        if is_a_curve {
            match UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 2)?.value {
                0 => {
                    typev = PrcCompressedCurveType::PRC_HCG_Line as u32;
                    e = ComprCurv(PrcCompressedCurveType::PRC_HCG_Line)
                }
                1 => {
                    typev = PrcCompressedCurveType::PRC_HCG_Circle as u32;
                    e = ComprCurv(PrcCompressedCurveType::PRC_HCG_Circle)
                }
                2 => {
                    typev = PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve as u32;
                    e = ComprCurv(PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve)
                }
                3 => match UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 2)?.value {
                    0 => {
                        typev = PrcCompressedCurveType::PRC_HCG_Ellipse as u32;
                        e = ComprCurv(PrcCompressedCurveType::PRC_HCG_Ellipse)
                    }
                    1 => {
                        typev = PrcCompressedCurveType::PRC_HCG_CompositeCurve as u32;
                        e = ComprCurv(PrcCompressedCurveType::PRC_HCG_CompositeCurve)
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "CompressedEntityType: unknown B pattern!",
                        ));
                    }
                },
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "CompressedEntityType: unknown A pattern!",
                    ));
                }
            };
        } else {
            typev = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 4)?.value;
            e = ComprFace(PrcCompressedFaceType::try_from(typev).unwrap());
        }
        //dbg!(&e);
        let rv = CompressedEntityType {
            value: typev,
            is_a_curve,
            aid: e,
        };
        //dbg!(rv);
        Ok(rv)
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_bit(self.is_a_curve)?;
        if self.is_a_curve {
            panic!("not implemented yet")
        } else {
            UnsignedIntegerWithVariableBitNumber { value: self.value }.to_writer(w, 4)?;
        }
        Ok(())
    }
    #[allow(non_snake_case)]
    pub fn is_PRC_HCG_NewLoop(&self) -> bool {
        !self.is_a_curve && self.value == PrcCompressedFaceType::PRC_HCG_NewLoop as u32
    }
    #[allow(non_snake_case)]
    pub fn is_PRC_HCG_EndLoop(&self) -> bool {
        !self.is_a_curve && self.value == PrcCompressedFaceType::PRC_HCG_EndLoop as u32
    }
}
impl fmt::Debug for CompressedEntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.is_a_curve {
            let e = PrcCompressedFaceType::try_from(self.value).unwrap();
            write!(
                f,
                "CompressedEntityType(value: {} ({}), is_a_curve: {}, {})",
                e, e as u32, self.is_a_curve, self.aid
            )
        } else {
            let e = PrcCompressedCurveType::try_from(self.value).unwrap();
            write!(
                f,
                "CompressedEntityType(value: {} ({}), is_a_curve: {}, {})",
                e, e as u32, self.is_a_curve, self.aid
            )
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct FloatAsBytes {
    pub value: f32,
}
impl FloatAsBytes {
    pub fn from_reader<R: BitRead>(rdr: &mut R) -> io::Result<Self> {
        #[allow(non_camel_case_types)]
        #[allow(non_snake_case)]
        #[derive(Clone, Copy)]
        #[repr(C/*, packed*/)]
        #[repr(align(4))]
        union f2u {
            pub f: f32,
            pub bytes: [u8; 4],
        }
        use std::mem;
        extern crate static_assertions as sa;
        sa::const_assert_eq!(4, mem::size_of::<f2u>());
        sa::const_assert_eq!(4, mem::align_of::<f2u>());

        let mut f2u: f2u = unsafe { mem::zeroed() };
        for i in 0..4 {
            unsafe {
                f2u.bytes[i] = UnsignedCharacter::from_reader(rdr)?.value;
            }
        }

        Ok(FloatAsBytes {
            value: unsafe { f2u.f },
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        #[allow(non_camel_case_types)]
        #[allow(non_snake_case)]
        #[derive(Clone, Copy)]
        #[repr(C/*, packed*/)]
        #[repr(align(4))]
        union f2u {
            pub f: f32,
            pub bytes: [u8; 4],
        }
        use std::mem;
        extern crate static_assertions as sa;
        sa::const_assert_eq!(4, mem::size_of::<f2u>());
        sa::const_assert_eq!(4, mem::align_of::<f2u>());

        let f2u: f2u = f2u { f: self.value };

        for i in 0..4 {
            UnsignedCharacter {
                value: unsafe { f2u.bytes[i] },
            }
            .to_writer(w)?;
        }
        Ok(())
    }
}
impl PartialEq for FloatAsBytes {
    fn eq(&self, other: &Self) -> bool {
        self.value.total_cmp(&other.value) == std::cmp::Ordering::Equal
    }
}
impl Eq for FloatAsBytes {}
impl fmt::Debug for FloatAsBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CharacterArray {
    pub a: Vec<i8>,
}
impl CharacterArray {
    pub fn from_reader<R: BitRead>(r: &mut R, num_bits_per_elem: u8) -> io::Result<Self> {
        let has_is_compressed_bit = true;
        //panic!("{}: Not implemented!", function!());
        let a = crate::prc_huffman::read_huffman_to_element_array_i8(
            r,
            has_is_compressed_bit,
            num_bits_per_elem,
            true,
        )?;

        Ok(Self { a })
    }
    pub fn from_reader2<R: BitRead>(
        r: &mut R,
        has_is_compressed_bit: bool,
        num_bits_per_elem: u8,
        is_compressed_dv: bool,
    ) -> io::Result<Self> {
        let a = crate::prc_huffman::read_huffman_to_element_array_i8(
            r,
            has_is_compressed_bit,
            num_bits_per_elem,
            is_compressed_dv,
        )?;
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        _num_bits_per_elem: u8,
    ) -> std::io::Result<()> {
        panic!("{}: Not implemented!", function!());
        Ok(())
    }
}
impl fmt::Debug for CharacterArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "CharacterArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "CharacterArray {} elements", self.a.len()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct ShortArray {
    pub a: Vec<i16>,
}
impl ShortArray {
    pub fn from_reader<R: BitRead>(r: &mut R, num_bits_per_elem: u8) -> io::Result<Self> {
        let has_is_compressed_bit = true;
        let a = crate::prc_huffman::read_huffman_to_element_array_i16(
            r,
            has_is_compressed_bit,
            num_bits_per_elem,
            true,
        )?;

        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        _num_bits_per_elem: u8,
    ) -> std::io::Result<()> {
        panic!("{}: Not implemented!", function!());
        Ok(())
    }
}
impl fmt::Debug for ShortArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "ShortArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "ShortArray {} elements", self.a.len()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CompressedIntegerArray {
    pub a: Vec<i32>,
}
impl CompressedIntegerArray {
    pub fn from_reader<R: BitRead>(_rdr: &mut R) -> io::Result<Self> {
        let has_is_compressed_bit = true;
        let num_bits_used_to_store_ints =
            CharacterArray::from_reader2(_rdr, has_is_compressed_bit, 6, true)?.a;
        let mut a: Vec<i32> = Vec::with_capacity(num_bits_used_to_store_ints.len());
        for i in 0..num_bits_used_to_store_ints.len() {
            let num_bits_in_int = num_bits_used_to_store_ints[i] as u32;
            a.push(IntegerWithVariableBitNumber::from_reader(_rdr, num_bits_in_int)?.value);
        }
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, _w: &mut W) -> std::io::Result<()> {
        panic!("{}: Not implemented!", function!());
        Ok(())
    }
}
impl fmt::Debug for CompressedIntegerArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "CompressedIntegerArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "CompressedIntegerArray {} elements", self.a.len()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CompressedIndiceArray {
    pub a: Vec<i32>,
}
impl CompressedIndiceArray {
    pub fn from_reader<R: BitRead>(r: &mut R) -> io::Result<Self> {
        let has_is_compressed_bit = true;
        let num_bits_used_to_store_chars = 6;
        CompressedIndiceArray::from_reader2(
            r,
            has_is_compressed_bit,
            num_bits_used_to_store_chars,
            true,
        )
    }
    pub fn from_reader2<R: BitRead>(
        r: &mut R,
        has_is_compressed_bit: bool,
        num_bits_used_to_store_chars: u8,
        is_compressed_dv: bool,
    ) -> io::Result<Self> {
        let diff_num_bits_used_to_store_ints = CharacterArray::from_reader2(
            r,
            has_is_compressed_bit,
            num_bits_used_to_store_chars,
            is_compressed_dv,
        )?
        .a;
        let num_elements = diff_num_bits_used_to_store_ints.len();
        if num_elements == 0 {
            return Ok(Self { a: Vec::new() });
        }

        if false {
            let min_value = diff_num_bits_used_to_store_ints.iter().min();
            let max_value = diff_num_bits_used_to_store_ints.iter().max();
            match (min_value, max_value) {
                (Some(min), Some(max)) => println!(
                    "CompressedIndiceArray {} elements in diff_num_bits_used_to_store_ints, range: [{}, {}]",
                    num_elements, min, max
                ),
                (_, _) => (),
            }
        }

        let mut pc_array: Vec<i8> = Vec::with_capacity(num_elements);
        pc_array.push(diff_num_bits_used_to_store_ints[0] as i8);
        let mut c_bit_count = pc_array[0];
        let mut pi_array: Vec<i32> = Vec::with_capacity(num_elements);
        pi_array.push(
            IntegerWithVariableBitNumber::from_reader(r, c_bit_count as u32)
                .unwrap()
                .value,
        );
        for i in 1..diff_num_bits_used_to_store_ints.len() {
            pc_array.push(diff_num_bits_used_to_store_ints[i] as i8);

            c_bit_count += pc_array[i];
            let ival = IntegerWithVariableBitNumber::from_reader(r, c_bit_count as u32)
                .unwrap()
                .value;
            pi_array.push(ival + pi_array[i - 1]);
        }

        if false {
            let min_value = pi_array.iter().min();
            let max_value = pi_array.iter().max();
            match (min_value, max_value) {
                (Some(min), Some(max)) => println!(
                    "CompressedIndiceArray {} elements in pi_array, range: [{}, {}]",
                    num_elements, min, max
                ),
                (_, _) => (),
            }
        }
        Ok(Self { a: pi_array })
    }
    /// the indices are always positive at input.
    pub fn to_writer<W: BitWrite + ?Sized>(&self, _w: &mut W) -> std::io::Result<()> {
        panic!("{}: Not implemented!", function!());
        Ok(())
    }
}
impl fmt::Debug for CompressedIndiceArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "CompressedIndiceArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "CompressedIndiceArray {} elements", self.a.len()),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CompressedIndiceArrayWithoutBit {
    pub a: Vec<i32>,
}
impl CompressedIndiceArrayWithoutBit {
    pub fn from_reader<R: BitRead>(r: &mut R, is_compressed_dv: bool) -> io::Result<Self> {
        let has_is_compressed_bit = false;
        let num_bits_used_to_store_chars = 6;
        let a = CompressedIndiceArray::from_reader2(
            r,
            has_is_compressed_bit,
            num_bits_used_to_store_chars,
            is_compressed_dv,
        )?
        .a;
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        _is_compressed_dv: bool,
    ) -> std::io::Result<()> {
        panic!("{}: Not implemented!", function!());
        Ok(())
    }
}
impl fmt::Debug for CompressedIndiceArrayWithoutBit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "CompressedIndiceArrayWithoutBit {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(
                f,
                "CompressedIndiceArrayWithoutBit {} elements",
                self.a.len()
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct DoubleWithVariableBitNumber {
    value: f64,
    #[allow(unused)]
    num_bits: u32, // TODO: remove, only needed for debugging
    #[allow(unused)]
    tolerance: f64, // TODO: remove, only needed for debugging
}
impl DoubleWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(
        _rdr: &mut R,
        num_bits: u32,
        tolerance: f64,
    ) -> io::Result<Self> {
        let neg = _rdr.read_bit()?;

        let mut u_temp_value = 0;
        for u in 0..(num_bits - 1) {
            let exp = num_bits - 2 - u;
            let thres = 1 << exp; // U
            let b = _rdr.read_bit()?;
            if b {
                u_temp_value += thres;
            }
            //std::cout << u << " " << u_temp_value << " " << thres << " " << exp << " b" << (u_temp_value>= thres) << b << std::endl;
        }
        let value = (u_temp_value as f64) * tolerance * (if neg { -1.0 } else { 1.0 });
        Ok(Self {
            value,
            num_bits,
            tolerance,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        assert!(num_bits > 1);
        assert!(tolerance > 0.0001);
        let _ = _w.write_bit(self.value < 0.0)?;
        let mut u_temp_value = (self.value.abs() / tolerance) as u32;
        let test = self.value.abs() / tolerance - u_temp_value as f64;
        if test > 0.5 {
            u_temp_value += 1;
        }

        for u in 0..(num_bits - 1) {
            let exp = num_bits - 2 - u;
            let thres = 1 << exp;
            if u_temp_value >= thres {
                let _ = _w.write_bit(true)?;
                u_temp_value -= thres;
            } else {
                let _ = _w.write_bit(false)?;
            }
        }
        Ok(())
    }
}
impl PartialEq for DoubleWithVariableBitNumber {
    fn eq(&self, other: &Self) -> bool {
        self.value.total_cmp(&other.value) == std::cmp::Ordering::Equal
    }
}
impl Eq for DoubleWithVariableBitNumber {}
impl fmt::Debug for DoubleWithVariableBitNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct Point3DWithVariableBitNumber {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[allow(unused)]
    num_bits: u32, // TODO: remove, only needed for debugging
    #[allow(unused)]
    tolerance: f64, // TODO: remove, only needed for debugging
}
impl Point3DWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(
        _rdr: &mut R,
        num_bits: u32,
        tolerance: f64,
    ) -> io::Result<Self> {
        assert!(num_bits > 1);
        assert!(num_bits <= 30);
        //assert!(tolerance > 0.00000001);
        assert!(tolerance > 0.0);

        let xi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
        let yi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
        let zi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
        let x = (xi as f64 - 0.5) * tolerance;
        let y = (yi as f64 - 0.5) * tolerance;
        let z = (zi as f64 - 0.5) * tolerance;

        Ok(Self {
            x,
            y,
            z,
            num_bits,
            tolerance,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        assert!(num_bits <= 30);
        // https://github.com/pdf-association/pdf-issues/issues/581
        let xi = (self.x / tolerance + 0.5) as i32;
        let yi = (self.y / tolerance + 0.5) as i32;
        let zi = (self.z / tolerance + 0.5) as i32;
        let _ = UnsignedIntegerWithVariableBitNumber { value: num_bits }.to_writer(_w, 6)?;
        let _ = IntegerWithVariableBitNumber { value: xi }.to_writer(_w, num_bits)?;
        let _ = IntegerWithVariableBitNumber { value: yi }.to_writer(_w, num_bits)?;
        let _ = IntegerWithVariableBitNumber { value: zi }.to_writer(_w, num_bits)?;
        Ok(())
    }
}
impl PartialEq for Point3DWithVariableBitNumber {
    fn eq(&self, other: &Self) -> bool {
        self.x.total_cmp(&other.x) == std::cmp::Ordering::Equal
            && self.y.total_cmp(&other.y) == std::cmp::Ordering::Equal
            && self.z.total_cmp(&other.z) == std::cmp::Ordering::Equal
    }
}
impl Eq for Point3DWithVariableBitNumber {}

// bug: https://github.com/pdf-association/pdf-issues/issues/581
// bug: https://github.com/pdf-association/pdf-issues/issues/706
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct CompressedPoint {
    // TODO: use Point3DWithVariableBitNumber0 internally
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[allow(unused)]
    num_bits: u32, // TODO: remove, only needed for debugging
    #[allow(unused)]
    tolerance: f64, // TODO: remove, only needed for debugging
}
impl CompressedPoint {
    pub fn from_reader<R: BitRead>(
        _rdr: &mut R,
        //num_bits: u32,
        tolerance: f64,
    ) -> io::Result<Self> {
        //assert!(num_bits > 1);
        //assert!(tolerance > 0.00000001);
        assert!(tolerance > 0.0);
        let num_bits = UnsignedIntegerWithVariableBitNumber::from_reader(_rdr, 6)?.value;
        let x;
        let y;
        let z;
        if num_bits <= 30 {
            let xi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
            let yi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
            let zi = IntegerWithVariableBitNumber::from_reader(_rdr, num_bits)?.value;
            x = (xi as f64 - 0.5) * tolerance;
            y = (yi as f64 - 0.5) * tolerance;
            z = (zi as f64 - 0.5) * tolerance;
        } else {
            x = Double::from_reader(_rdr)?.value;
            y = Double::from_reader(_rdr)?.value;
            z = Double::from_reader(_rdr)?.value;
        }
        Ok(Self {
            x,
            y,
            z,
            num_bits,
            tolerance,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        //_num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        // https://github.com/pdf-association/pdf-issues/issues/581
        assert!(tolerance > 0.0);
        let xi = (self.x / tolerance + 0.5) as i32;
        let yi = (self.y / tolerance + 0.5) as i32;
        let zi = (self.z / tolerance + 0.5) as i32;
        let num_bits = get_number_of_bits_used_to_store_integer(max(xi, yi, zi));
        let _ = UnsignedIntegerWithVariableBitNumber { value: num_bits }.to_writer(_w, 6)?;
        if num_bits <= 30 {
            let _ = IntegerWithVariableBitNumber { value: xi }.to_writer(_w, num_bits)?;
            let _ = IntegerWithVariableBitNumber { value: yi }.to_writer(_w, num_bits)?;
            let _ = IntegerWithVariableBitNumber { value: zi }.to_writer(_w, num_bits)?;
        } else {
            let _ = Double { value: self.x }.to_writer(_w)?;
            let _ = Double { value: self.y }.to_writer(_w)?;
            let _ = Double { value: self.z }.to_writer(_w)?;
        }
        Ok(())
    }
}
impl PartialEq for CompressedPoint {
    fn eq(&self, other: &Self) -> bool {
        self.x.total_cmp(&other.x) == std::cmp::Ordering::Equal
            && self.y.total_cmp(&other.y) == std::cmp::Ordering::Equal
            && self.z.total_cmp(&other.z) == std::cmp::Ordering::Equal
    }
}
impl Eq for CompressedPoint {}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UncompressedBoolArray {
    pub a: Vec<bool>,
}
impl UncompressedBoolArray {
    pub fn from_reader<R: BitRead>(rdr: &mut R, num_bits: u32) -> io::Result<Self> {
        //println!("UncompressedBoolArray: {}", num_bits);
        let mut a: Vec<bool> = Vec::with_capacity(num_bits as usize);
        a.resize(num_bits as usize, false);
        for u in 0..a.len() {
            let b = rdr.read_bit()?;
            a[u] = b;
        }
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, _: u32) -> std::io::Result<()> {
        for u in 0..self.a.len() {
            let _ = w.write_bit(self.a[u])?;
        }
        Ok(())
    }
}
impl fmt::Debug for UncompressedBoolArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let min_value = self.a.iter().min();
        let max_value = self.a.iter().max();
        match (min_value, max_value) {
            (Some(min), Some(max)) => write!(
                f,
                "UncompressedBoolArray {} elements, range: [{}, {}]",
                self.a.len(),
                min,
                max
            ),
            (_, _) => write!(f, "UncompressedBoolArray {} elements", self.a.len()),
        }
    }
}


#[derive(Debug, PartialEq, Eq)]
pub struct PRCHeader {
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
    pub num_file_struts: u32,
    pub fsi: Vec<PRCFileStructureInformation>,
    pub mf: Vec<u8>,
    pub uncompr_files: Vec<Vec<u8>>,
}

impl PRCHeader {
    pub fn from_reader(mut rdr: impl Read + Seek, file_size_bytes: usize) -> io::Result<Self> {
        debug_time!("{:?}", "PRCHeader::from_reader");
        let mut magic: [u8; 3] = [0; 3];
        rdr.read_exact(&mut magic)?;
        if magic[0] != b'P' || magic[1] != b'R' || magic[2] != b'C' {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid magic"));
        }

        let verread = rdr.read_u32::<LittleEndian>()?;
        let verauth = rdr.read_u32::<LittleEndian>()?;
        let uuid0 = rdr.read_u32::<LittleEndian>()?;
        let uuid1 = rdr.read_u32::<LittleEndian>()?;
        let uuid2 = rdr.read_u32::<LittleEndian>()?;
        let uuid3 = rdr.read_u32::<LittleEndian>()?;
        let uuida0 = rdr.read_u32::<LittleEndian>()?;
        let uuida1 = rdr.read_u32::<LittleEndian>()?;
        let uuida2 = rdr.read_u32::<LittleEndian>()?;
        let uuida3 = rdr.read_u32::<LittleEndian>()?;
        let num_file_struts = rdr.read_u32::<LittleEndian>()?;

        info!("Version for reading: {}", verread);
        info!("Authoring version: {}", verauth);
        info!("Number of file sections: {}", num_file_struts);

        let mut fsi = Vec::new();
        for i in 0..num_file_struts {
            let fsii = PRCFileStructureInformation::from_reader(&mut rdr);
            let b = fsii.as_ref();
            debug!("fsi {}: {}", i, b.unwrap().offsets.len());
            assert_eq!(
                fsii.as_ref().unwrap().offsets.len(),
                PrcSectionKind::ExtraGeometry as usize + 2
            );
            fsi.push(fsii?);
        }

        let mf_start_offset = rdr.read_u32::<LittleEndian>()?;
        let mf_end_offset = rdr.read_u32::<LittleEndian>()?;
        let num_uncompr_files = rdr.read_u32::<LittleEndian>()?;

        //let file_size = rdr.stream_len();
        let mf_size = mf_end_offset - mf_start_offset;
        debug!(
            "mf compressed offset: [{},{}], size: {}",
            mf_start_offset, mf_end_offset, mf_size
        );
        debug!("num_uncompr_files: {}", num_uncompr_files);
        let mut uncompr_files: Vec<Vec<u8>> = Vec::with_capacity(num_uncompr_files as usize);
        for i in 0..num_uncompr_files {
            let num_bytes = rdr.read_u32::<LittleEndian>()?;
            debug!("uncompressed file {}: {}", i, num_bytes);
            let mut bytes: Vec<u8> = vec![0; num_bytes as usize];
            rdr.read_exact(&mut bytes)?;
            uncompr_files.push(bytes);
        }

        let mut mf_compr: Vec<u8> = vec![0; mf_size as usize];
        rdr.seek(std::io::SeekFrom::Start(mf_start_offset as u64))?;
        rdr.read_exact(&mut mf_compr)?;
        //let mf0 = inflate_bytes(&mf_compr);
        let mf = decompress(&mf_compr).unwrap();
        debug!("mf uncompressed {} -> {}", mf_compr.len(), mf.len());

        let mut compressed_sections: Vec<Vec<Vec<u8>>> =
            Vec::with_capacity(num_file_struts as usize);
        compressed_sections.resize(
            num_file_struts as usize,
            Vec::with_capacity(PrcSectionKind::ExtraGeometry as usize + 1),
        );
        for i in 0..num_file_struts as usize {
            fsi[i]
                .sections
                .resize(PrcSectionKind::ExtraGeometry as usize + 1, Vec::new());
            compressed_sections[i].resize(PrcSectionKind::ExtraGeometry as usize + 1, Vec::new());
            for j in 1..fsi[i as usize].offsets.len() {
                let start_offset = fsi[i as usize].offsets[j];
                let end_offset;
                if (j + 1) < fsi[i as usize].offsets.len() {
                    end_offset = fsi[i as usize].offsets[j + 1];
                } else if (i + 1) < num_file_struts as usize {
                    end_offset = fsi[i as usize + 1].offsets[0];
                } else {
                    end_offset = std::cmp::min(mf_start_offset, file_size_bytes as u32);
                }

                let size = end_offset - start_offset;
                //debug!("{} {} [{},{}] {}", i, j, start_offset, end_offset, size);
                let mut section_compr: Vec<u8> = vec![0; size as usize];
                rdr.seek(std::io::SeekFrom::Start(fsi[i as usize].offsets[j] as u64))?;
                rdr.read_exact(&mut section_compr)?;
                compressed_sections[i][j - 1] = section_compr;
            }
            fsi[i as usize].offsets.clear();
        }

        // decompression could happen concurrently
        let parallel_decompression = false;
        let now = std::time::Instant::now();
        if !parallel_decompression {
            for (i, file_sections) in compressed_sections.iter().enumerate() {
                for (j, section_compressed) in file_sections.iter().enumerate() {
                    let section = decompress(&section_compressed).unwrap();
                    debug!(
                        "{}/{}: section uncompressed {} -> {}",
                        i,
                        j,
                        section_compressed.len(),
                        section.len()
                    );
                    fsi[i].sections[j] = section;
                }
                assert_eq!(
                    fsi[i].sections.len(),
                    PrcSectionKind::ExtraGeometry as usize + 1
                );
            }
        } else {
            let fsi_sections = compressed_sections
                .par_iter()
                .map(|file_sections| {
                    let sections = file_sections
                        .par_iter()
                        .map(|section_compressed| decompress(&section_compressed).unwrap())
                        .collect::<Vec<_>>();
                    assert_eq!(sections.len(), PrcSectionKind::ExtraGeometry as usize + 1);
                    sections
                })
                .collect::<Vec<_>>();
            for (i, file_sections) in fsi_sections.iter().enumerate() {
                fsi[i].sections = file_sections.to_vec();
            }
        }
        let elapsed = now.elapsed();
        debug!("Decompression took {:.2?}ms", elapsed.as_micros() as f64 / 1000.0);

        Ok(PRCHeader {
            verread,
            verauth,
            uuid0,
            uuid1,
            uuid2,
            uuid3,
            uuida0,
            uuida1,
            uuida2,
            uuida3,
            num_file_struts,
            fsi,
            mf,
            uncompr_files,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PRCFileStructureInformation {
    pub uuid0: u32,
    pub uuid1: u32,
    pub uuid2: u32,
    pub uuid3: u32,

    pub reserved: u32,
    pub offsets: Vec<u32>,
    pub sections: Vec<Vec<u8>>,
}

impl PRCFileStructureInformation {
    fn from_reader(mut rdr: impl Read) -> io::Result<Self> {
        let uuid0 = rdr.read_u32::<LittleEndian>()?;
        let uuid1 = rdr.read_u32::<LittleEndian>()?;
        let uuid2 = rdr.read_u32::<LittleEndian>()?;
        let uuid3 = rdr.read_u32::<LittleEndian>()?;
        let reserved = rdr.read_u32::<LittleEndian>()?;
        let num_offsets = rdr.read_u32::<LittleEndian>()?;
        let mut offsets = Vec::new();
        for _n in 0..num_offsets {
            let tmp = rdr.read_u32::<LittleEndian>()?;
            offsets.push(tmp);
        }
        let sections = Vec::new();

        Ok(PRCFileStructureInformation {
            uuid0,
            uuid1,
            uuid2,
            uuid3,
            reserved,
            offsets,
            sections,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitstream_io::{BigEndian, BitWriter};
    //use crate::common::PrcParsingContext;
    use std::fs::File;
    use std::io::Read;
    use std::io::{BufRead, Cursor, Write};
    use std::ops::Add;

    /// fill partial byte at the end
    fn fill_partial_byte_at_end<W: BitWrite + ?Sized>(w: &mut W) -> std::io::Result<usize> {
        let mut trailing_bits: usize = 0;
        while !w.byte_aligned() {
            w.write_bit(false)?;
            trailing_bits += 1;
        }
        Ok(trailing_bits)
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
    fn io_bool() {
        let mut bytes = vec![];

        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::LittleEndian);
            assert!(w.write_bit(false).is_ok());
            assert!(w.write_bit(true).is_ok());
            assert!(w.write_bit(true).is_ok());
            assert!(w.write_bit(true).is_ok());
            assert!(w.write_bit(false).is_ok());
            assert!(w.write_bit(false).is_ok());
            assert!(w.write_bit(false).is_ok());
            assert!(w.write_bit(true).is_ok());
        }
        assert_eq!(bytes, vec![0b1000_1110]);
        assert_eq!(bytes.len(), 1 as usize);

        bytes.clear();
        assert_eq!(bytes.len(), 0 as usize);

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut b: Boolean = Boolean { value: true };
            let _ = b.to_writer(&mut w);
            b = Boolean { value: false };
            let _ = b.to_writer(&mut w).unwrap();

            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 1 as usize);
        assert_eq!(bytes, vec![0b1000_0000]);
        println!("v={:#?}", bytes);

        //let mut ctx: PrcParsingContext = Default::default();
        let mut reader = BitReader::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
        let mut b: bool = Boolean::from_reader(&mut reader).unwrap().value;
        assert_eq!(b, true);
        b = Boolean::from_reader(&mut reader).unwrap().value;
        assert_eq!(b, false);
    }

    #[test]
    fn io_uchar() {
        let mut bytes = vec![];

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::LittleEndian);
            let mut uc = UnsignedCharacter { value: 125u8 };
            let _ = uc.to_writer(&mut w);
            uc.value = 0u8;
            let _ = uc.to_writer(&mut w);
            uc.value = 255u8;
            let _ = uc.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 3 as usize);

        //let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&mut bytes), BigEndian);
        assert_eq!(
            UnsignedCharacter::from_reader(&mut r).unwrap().value as u8,
            125 as u8
        );
        assert_eq!(
            UnsignedCharacter::from_reader(&mut r).unwrap().value as u8,
            0 as u8
        );
        assert_eq!(
            UnsignedCharacter::from_reader(&mut r).unwrap().value as u8,
            255 as u8
        );
    }

    #[test]
    fn io_ushort() {
        let mut bytes = vec![];
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut us = UnsignedShort {
                value: (3_u8 as u16) << 8 | 1_u8 as u16,
            };
            let _ = us.to_writer(&mut w);
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 2_usize);
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(
            UnsignedShort::from_reader(&mut r).unwrap().value,
            0b00000011_00000001
        );
    }

    #[test]
    fn io_only_uint() {
        let mut bytes = vec![];

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut ui = UnsignedInteger { value: 125 };
            let _ = ui.to_writer(&mut w);
            ui.value = 0;
            let _ = ui.to_writer(&mut w);
            ui.value = 1239255;
            let _ = ui.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 5 as usize);

        //let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 125);
        assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 0);
        assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 1239255);
    }

    #[test]
    fn io_string() {
        let mut bytes = vec![];
        let ss = std::string::String::from(
            "Abracadabra order matters:77 CCCCitStream last to initialized last bla-bla 1234",
        );
        let s = String { value: ss.clone() };
        let semp: String = Default::default();

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            semp.to_writer(&mut w).unwrap();
            s.to_writer(&mut w).unwrap();

            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 81);

        //let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(String::from_reader(&mut r).unwrap().value, "");
        assert_eq!(String::from_reader(&mut r).unwrap().value, ss);
    }

    #[test]
    fn io_only_int() {
        let mut bytes = vec![];

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut i = Integer { value: 125 };
            let _ = i.to_writer(&mut w);
            i.value = 0;
            let _ = i.to_writer(&mut w);
            i.value = -1239255;
            let _ = i.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 5 as usize);

        //let mut ctx: PrcParsingContext = Default::default();
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(Integer::from_reader(&mut r).unwrap().value, 125);
        assert_eq!(Integer::from_reader(&mut r).unwrap().value, 0);
        assert_eq!(Integer::from_reader(&mut r).unwrap().value, -1239255);
    }

    #[test]
    fn read_ints() {
        let path = std::env::current_dir().unwrap();
        println!("The current directory is {}", path.display());
        let bytes_external =
            get_file_as_byte_vec(&std::string::String::from("testdata/read_ints.bin"));
        assert_eq!(bytes_external.len(), 808992 as usize);

        let n: u32 = 66002;
        let mut r = BitReader::endian(Cursor::new(&bytes_external), BigEndian);
        for i in 0..n {
            let u1 = UnsignedInteger::from_reader(&mut r).unwrap().value;
            let i1 = Integer::from_reader(&mut r).unwrap().value;
            let i2 = Integer::from_reader(&mut r).unwrap().value;
            let u2 = UncompressedUnsignedInteger::from_reader(&mut r)
                .unwrap()
                .value;

            assert_eq!(i, u1);
            assert_eq!(i, i1 as u32);
            assert_eq!(i, -i2 as u32);
            assert_eq!(i, u2);
        }

        let mut bytes = vec![];
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = UnsignedInteger { value: i }.to_writer(&mut w).unwrap();
                let _ = Integer { value: i as i32 }.to_writer(&mut w).unwrap();
                let _ = Integer { value: -(i as i32) }.to_writer(&mut w).unwrap();
                let _ = UncompressedUnsignedInteger { value: i }
                    .to_writer(&mut w)
                    .unwrap();
            }
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes_external, bytes);
    }

    #[test]
    fn io_uint_vbr() {
        let n: u32 = 31966002;

        let mut bytes = vec![];
        let trailing_bits: u64;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = NumberOfBitsThenUnsignedInteger { value: i }
                    .to_writer(&mut w)
                    .unwrap();
            }
            trailing_bits = fill_partial_byte_at_end(&mut w)
                .expect("failed to fill partial byte at end") as u64;
        }
        assert_eq!(115678204, bytes.len());

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        for i in 0..n {
            let u1 = NumberOfBitsThenUnsignedInteger::from_reader(&mut r)
                .unwrap()
                .value;

            assert_eq!(i, u1);
        }
        assert_eq!(
            115678204 as u64 * 8,
            r.position_in_bits().unwrap() + trailing_bits
        );
    }

    #[test]
    fn write_one_double() {
        let mut bytes = vec![];

        //let value: f64 = -296.37;
        //let value: f64 = -485.07;
        //let num_bits: usize = 60;
        let value = 32768.099999999998544808477163314819;
        let num_bits = 59;
        let trailing_bits: usize;
        let num_bits_encoded;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            Double { value }.to_writer(&mut w).unwrap();
            trailing_bits =
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
            num_bits_encoded = bytes.len() * 8 - trailing_bits;
            println!("{}", to_bits_str(&bytes, num_bits_encoded));
        }
        println!("bits: {}", num_bits_encoded);
        assert_eq!(num_bits as usize, num_bits_encoded);

        println!("bytes: {}", bytes.len());
        //assert_eq!(8 as usize, bytes.len());

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let recovered = Double::from_reader(&mut r).unwrap().value;
        println!("num bits read: {}", r.position_in_bits().unwrap());
        assert_eq!(num_bits as u64, r.position_in_bits().unwrap());
        assert_eq!(value, recovered);
    }

    #[test]
    fn io_double() {
        let n = 6002;

        let mut bytes = vec![]; // PRC binary serialization data

        //let mut s: std::string::String = std::string::String::new(); // serde serialization data
        let mut s: Vec<std::string::String> = vec![];

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let u = UnsignedInteger { value: i };
                let d1 = Double {
                    value: i as f64 * 1.15,
                };
                let d2 = Double {
                    value: i as f64 * -1.11,
                };

                u.to_writer(&mut w).unwrap();
                d1.to_writer(&mut w)
                    .unwrap();
                d2.to_writer(&mut w)
                .unwrap();

                s.push(serde_json::to_string(&u).unwrap());
                s.push(serde_json::to_string(&d1).unwrap());
                s.push(serde_json::to_string(&d2).unwrap());
                // s = s + &serde_json::to_string(&u).unwrap();
                // s = s + &serde_json::to_string(&d1).unwrap();
                // s = s + &serde_json::to_string(&d2).unwrap();
            }
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }

        println!("bytes: {}", bytes.len());
        assert_eq!(bytes.len(), 95340 as usize);

        //assert_eq!(s.len(), 177612usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        for i in 0..n {
            let ui = UnsignedInteger::from_reader(&mut r).unwrap().value;
            assert_eq!(i, ui);
            let ui: u32 = serde_json::from_str(&s[i as usize*3 + 0]).unwrap();
            assert_eq!(i, ui);

            let mut reference = i as f64 * 1.15;
            let recovered = Double::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
            let recovered: Double = serde_json::from_str(&s[i as usize*3 + 1]).unwrap();
            assert!((reference - recovered.value).abs() < 1E-9);

            reference = i as f64 * -1.11;
            let recovered = Double::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
            let recovered: Double = serde_json::from_str(&s[i as usize*3 + 2]).unwrap();
            assert!((reference - recovered.value).abs() < 1E-9);
        }
    }

    #[test]
    fn io_float() {
        let n = 66002;

        let mut bytes = vec![];
        let num_trailing_padding_bits;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                UnsignedInteger { value: i }.to_writer(&mut w).unwrap();
                FloatAsBytes {
                    value: i as f32 * 1.15,
                }
                .to_writer(&mut w)
                .unwrap();
                FloatAsBytes {
                    value: i as f32 * -1.11,
                }
                .to_writer(&mut w)
                .unwrap();
            }
            num_trailing_padding_bits =
                fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }

        println!("bytes: {}", bytes.len());
        assert_eq!(bytes.len(), 685006 as usize);
        assert_eq!(bytes[685006 - 1 - 0], 142);
        assert_eq!(bytes[685006 - 1 - 1], 31);
        assert_eq!(bytes[685006 - 1 - 2], 45);
        assert_eq!(bytes[685006 - 1 - 3], 28);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        for i in 0..n {
            let ui = UnsignedInteger::from_reader(&mut r).unwrap().value;
            assert_eq!(i, ui);
            let mut reference = i as f32 * 1.15;
            let mut recovered = FloatAsBytes::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
            reference = i as f32 * -1.11;
            recovered = FloatAsBytes::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
        }

        assert_eq!(
            (bytes.len() * 8 - num_trailing_padding_bits) as u64,
            r.position_in_bits().unwrap()
        );
    }

    #[test]
    fn read_doubles() {
        let path = std::env::current_dir().unwrap();
        println!("[read_doubles] The current directory is {}", path.display());
        let bytes_external =
            get_file_as_byte_vec(&std::string::String::from("testdata/read_doubles.bin"));
        assert_eq!(bytes_external.len(), 95340 as usize);

        let n: u32 = 6002;
        let mut r = BitReader::endian(Cursor::new(&bytes_external), BigEndian);
        for i in 0..n {
            let ui = UnsignedInteger::from_reader(&mut r).unwrap().value;
            let d1 = Double::from_reader(&mut r).unwrap().value;
            let d2 = Double::from_reader(&mut r).unwrap().value;

            assert_eq!(i, ui);
            assert_eq!(i as f64 * 1.15, d1);
            assert_eq!(i as f64 * -1.11, d2);
        }

        let mut bytes = vec![];
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = UnsignedInteger { value: i }.to_writer(&mut w).unwrap();
                let _ = Double {
                    value: i as f64 * 1.15,
                }
                .to_writer(&mut w)
                .unwrap();
                let _ = Double {
                    value: i as f64 * -1.11,
                }
                .to_writer(&mut w)
                .unwrap();
            }
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes_external.len(), bytes.len());
        assert_eq!(bytes_external, bytes);
    }

    #[allow(non_camel_case_types)]
    #[repr(align(8))]
    union d2u {
        d: f64,
        u: u64,
    }

    fn to_u64(d: f64) -> u64 {
        unsafe {
            let conv: d2u = d2u { d };
            conv.u
        }
    }

    fn to_bits_str(bytes: &Vec<u8>, n: usize) -> std::string::String {
        let mut s = std::string::String::new();
        for i in 0..n {
            let byte_index = i / 8;
            let bit_index = i % 8;
            let uc = bytes[byte_index];
            let bit = ((uc >> (7 - bit_index)) & 1) != 0;
            s = format!("{}{}", s, bit as u8);
        }
        s
    }

    fn from_bits_str(st: &str) -> Vec<u8> {
        let mut v: Vec<u8> = vec![];
        let mut uc: u8 = 0;

        let mut n = 0; // number of bits collected in uc
        for s in st.chars() {
            if s != '0' && s != '1' {
                panic!("Unexpected bit!")
            }
            let bit: u8 = if s == '1' { 1 } else { 0 };
            uc |= bit << (7 - n);
            n += 1;

            if n == 8 {
                v.push(uc);
                n = 0;
                uc = 0;
            }
        }
        if n > 0 {
            v.push(uc);
        }
        v
    }

    #[allow(unused)]
    //#[test]
    fn generate_csv_doubles() {
        let n = 1966002;

        let f = std::fs::File::create("doubles_rust.csv").expect("Should be able to create file");
        let mut br = std::io::BufWriter::new(f);

        for i in 0..n {
            {
                let mut bytes = vec![];
                let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
                let d = i as f64 * 1.15;
                Double { value: d }.to_writer(&mut w).unwrap();
                let num_trailing_passing_bits =
                    fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
                let bits_used = bytes.len() * 8 - num_trailing_passing_bits;
                //let sbits = bytes.into_iter().map(|d| format!("{:b}", d)).collect::<Vec<_>>().join("");
                let sbits = to_bits_str(&bytes, bits_used);
                let _ = br
                    .write_fmt(format_args!(
                        "{:.30}\t{}\t{}\t{}\n",
                        d,
                        to_u64(d),
                        bits_used,
                        sbits
                    ))
                    .unwrap();
            }

            {
                let mut bytes = vec![];
                let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
                let d = i as f64 * -1.11;
                Double { value: d }.to_writer(&mut w).unwrap();
                let num_trailing_passing_bits =
                    fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
                let bits_used = bytes.len() * 8 - num_trailing_passing_bits;
                //let sbits = bytes.into_iter().map(|d| format!("{:b}", d)).collect::<Vec<_>>().join("");
                let sbits = to_bits_str(&bytes, bits_used);
                let _ = br
                    .write_fmt(format_args!(
                        "{:.30}\t{}\t{}\t{}\n",
                        d,
                        to_u64(d),
                        bits_used,
                        sbits
                    ))
                    .unwrap();
            }
        }
    }

    #[allow(unused)]
    //#[test]
    fn verify_csv_doubles() {
        let f = std::fs::File::open("doubles_rust.csv").expect("Should be able to open file");
        let br = std::io::BufReader::new(f);

        let mut n: usize = 0;
        for line in br.lines() {
            let line = line.expect("Should be able to read line");

            let ll = (line.as_str()).split("\t").collect::<Vec<&str>>();
            assert_eq!(ll.len(), 4);

            let d: f64 = ll[0].parse::<f64>().expect("Should be able to parse f64");
            let u: u64 = ll[1].parse::<u64>().expect("Should be able to parse u64");
            let num_bits: usize = ll[2].parse().expect("Should be able to parse usize");
            let bytes_external = from_bits_str(ll[3]);

            let mut r = BitReader::endian(Cursor::new(&bytes_external), BigEndian);
            let d1 = match Double::from_reader(&mut r) {
                Ok(d) => d.value,
                _ => panic!(
                    "failed to read double @ {:.30} {} {} {}",
                    d, u, num_bits, ll[3]
                ),
            };

            assert_eq!(d, d1);
            assert_eq!(r.position_in_bits().unwrap(), num_bits as u64);

            n += 1;
        }

        assert_eq!(1966002usize * 2, n);
    }

    #[test]
    fn io_userdata() {
        let mut bytes: Vec<u8> = vec![];
        let reference: UserData;
        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            let mut data = Vec::with_capacity(123);
            for i in 0..123 {
                data.push(i % 3 == 0);
            }
            reference = UserData { data };
            let _ = reference.to_writer(&mut w);
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 17usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let ud = UserData::from_reader(&mut r).unwrap();
        assert_eq!(123usize, ud.data.len());
        assert_eq!(reference, ud);
    }

    #[test]
    fn io_compressed_scalars() {
        let mut bytes: Vec<u8> = vec![];

        // UnsignedIntegerWithVariableBitNumber
        // DoubleWithVariableBitNumber
        // Point3DWithVariableBitNumber

        let n = 1000;
        let num_bits = 30;
        let tol = 0.01;

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = UnsignedIntegerWithVariableBitNumber { value: i }
                    .to_writer(&mut w, num_bits)
                    .unwrap();
                let _ = DoubleWithVariableBitNumber {
                    value: i as f64 * -1.11,
                    num_bits,
                    tolerance: tol,
                }
                .to_writer(&mut w, num_bits, tol)
                .unwrap();
                let _ = CompressedPoint {
                    x: i as f64 * -1.12,
                    y: i as f64 * 0.97,
                    z: i as f64 * 2.54,
                    num_bits,
                    tolerance: tol,
                }
                .to_writer(&mut w, /*num_bits,*/ tol)
                .unwrap();
            }
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        dbg!(bytes.len());
        assert_eq!(14983, bytes.len());
        //assert_eq!(bytes_external, bytes);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        for i in 0..n {
            let ui = UnsignedIntegerWithVariableBitNumber::from_reader(&mut r, num_bits)
                .unwrap()
                .value;
            assert_eq!(ui, i);
            let d = DoubleWithVariableBitNumber::from_reader(&mut r, num_bits, tol)
                .unwrap()
                .value;
            //println!("{}: {} {} {}", i, i as f64 * -1.11, d, (i as f64 * -1.11 - d).abs());
            //dbg!(i);
            assert!((i as f64 * -1.11 - d).abs() < tol);

            //let num_bits1 = UnsignedIntegerWithVariableBitNumber::from_reader(&mut r, 6).unwrap().value;
            let p3 = CompressedPoint::from_reader(&mut r, /*num_bits1,*/ tol).unwrap();
            //dbg!(p3.x - i as f64 * -1.12, tol);
            assert!((p3.x - i as f64 * -1.12).abs() < tol);
            assert!((p3.y - i as f64 * 0.97).abs() < tol);
            assert!((p3.z - i as f64 * 2.54).abs() < tol);
        }
    }

    #[test]
    fn io_uncompressed_arrays() {
        let mut bytes: Vec<u8> = vec![];

        let bools = vec![true, false, true, true, false, false, false, false, true, false, true, true, true];
        let n: usize = 13;
        assert_eq!(bools.len(), n);

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let _ = UncompressedBoolArray { a: bools.clone() }
                .to_writer(&mut w, 0)
                .unwrap();
            fill_partial_byte_at_end(&mut w).expect("failed to fill partial byte at end");
        }
        dbg!(bytes.len());
        assert_eq!(2, bytes.len());
        //assert_eq!(bytes_external, bytes);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let read_bools = UncompressedBoolArray::from_reader(&mut r, n as u32)
                .unwrap()
                .a;
        assert_eq!(bools, read_bools);
    }

    #[test]
    fn test_have_bbox() {
        assert_eq!(true, have_bbox(1));
        assert_eq!(true, have_bbox(2));
        assert_eq!(true, have_bbox(3));
        assert_eq!(false, have_bbox(4))
    }
}
