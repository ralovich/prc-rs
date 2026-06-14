// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![debugger_visualizer(gdb_script_file = "gdb_debugger_visualizer_prc.py")]
#![allow(unreachable_code)]
#![allow(unused)]

/// Built-in structures that are bit-aligned.
use crate::constants;
use crate::constants::*;
use crate::decompress::decompress;
use crate::double;
use crate::function;
use crate::indent;
use crate::prc_gen::{CompressedMultiplicitiesU, CompressedMultiplicitiesV};
use bitstream_io::{BitRead, BitReader, BitWrite};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error, info, trace, warn};
use measure_time::debug_time;
use num_enum::TryFromPrimitive;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub fn have_bbox(bounding_box_behavior: i8) -> bool {
    let bf = PrcBodyBoundingBoxBehaviorBitField::from_bytes([bounding_box_behavior as u8]);
    bf.PRC_BODY_BBOX_Evaluation() || bf.PRC_BODY_BBOX_Precise()
}

fn max(i: i32, j: i32, k: i32) -> i32 {
    std::cmp::max(std::cmp::max(i, j), k)
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

pub fn sum_up_u(mult: &Vec<CompressedMultiplicitiesU>) -> (Vec<u32>, u32) {
    fn get_multiplicity(mult: &Vec<CompressedMultiplicitiesU>, i: usize) -> u32 {
        if i == 0 {
            //assert!(!mult[i].multiplicity_is_stored);
            if !mult[i].multiplicity_is_not_stored {
                mult[i].multiplicity.unwrap().value
            } else {
                warn!("according to sdk9, return 1, might be wrong!");
                1
            }
        } else {
            if !mult[i].multiplicity_is_not_stored {
                mult[i].multiplicity.unwrap().value
            } else {
                get_multiplicity(mult, i - 1)
            }
        }
    }
    let mut flat = vec![];
    let mut accum = 0_u32;
    for i in 0..mult.len() {
        let m = get_multiplicity(mult, i);
        flat.push(m);
        accum += m;
    }
    (flat, accum)
}

pub fn sum_up_v(mult: &Vec<CompressedMultiplicitiesV>) -> (Vec<u32>, u32) {
    fn get_multiplicity(mult: &Vec<CompressedMultiplicitiesV>, i: usize) -> u32 {
        if i == 0 {
            //assert!(!mult[i].multiplicity_is_stored);
            if !mult[i].multiplicity_is_not_stored {
                mult[i].multiplicity.unwrap().value
            } else {
                warn!("according to sdk9, return 1, might be wrong!");
                1
            }
        } else {
            if !mult[i].multiplicity_is_not_stored {
                mult[i].multiplicity.unwrap().value
            } else {
                get_multiplicity(mult, i - 1)
            }
        }
    }
    let mut flat = vec![];
    let mut accum = 0_u32;
    for i in 0..mult.len() {
        let m = get_multiplicity(mult, i);
        flat.push(m);
        accum += m;
    }
    (flat, accum)
}

/// Current position in a seekable stream.
pub fn position<S: Seek>(rdr: &mut S) -> std::io::Result<u64> {
    rdr.seek(SeekFrom::Current(0))
}

pub fn read_bits<R: BitRead>(r: &mut R, num_bits: u8) -> std::io::Result<u8> {
    assert!(num_bits == 1 || num_bits == 3 || num_bits == 4 || num_bits == 8);
    let mut value: u8 = 0;
    for i in 0..num_bits {
        value <<= 1;
        let bit = r.read_bit()?;
        value |= bit as u8;
    }
    Ok(value)
}
pub fn write_bits<W: BitWrite + ?Sized>(w: &mut W, value: u8, num_bits: u8) -> std::io::Result<()> {
    assert!(num_bits == 1 || num_bits == 3 || num_bits == 4 || num_bits == 8);
    for i in 0..num_bits {
        let bit = 1 == (value >> (num_bits - 1 - i)) & 1;
        w.write_bit(bit)?;
    }
    Ok(())
}

#[derive(Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Boolean {
    pub value: bool,
}
impl Boolean {
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        Ok(Self {
            value: read_bits(rdr, 1)? != 0,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        write_bits(w, self.value as u8, 1)
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
                Ok(Boolean { value })
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
        Ok(Self {
            value: read_bits(rdr, 8)? as i8,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        write_bits(w, self.value as u8, 8)
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
        Ok(Self {
            value: read_bits(rdr, 8)?,
        })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        write_bits(w, self.value, 8)
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
        // let lo = rdr.read_to::<u8>().unwrap();
        // let hi = rdr.read_to::<u8>().unwrap();
        let lo = read_bits(rdr, 8)?;
        let hi = read_bits(rdr, 8)?;
        let value: u16 = (hi as u16) << 8 | lo as u16;
        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let lo = (self.value & 0xFF) as u8;
        let hi = (self.value >> 8) as u8;
        // w.write::<8, _>(lo)?;
        // w.write::<8, _>(hi)
        write_bits(w, lo, 8)?;
        write_bits(w, hi, 8)
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
        //while rdr.read_bit()? && i < 4 {
        while (read_bits(rdr, 1)? != 0) && i < 4 {
            //let ux8: u8 = rdr.read_to::<u8>()?;
            let ux8: u8 = read_bits(rdr, 8)?;
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
        let found_offsets =
            Self::search_and_seek_back(rdr, needle, max_offset_bits, max_found_count);
        dbg!(&found_offsets);
        if !found_offsets.is_empty() {
            if found_offsets[0] <= max_allowed_offset {
                rdr.seek_bits(SeekFrom::Current(found_offsets[0] as i64))?;
                let ui = Self::from_reader(rdr)?;
                return Ok(ui);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "[search_and_read did not find {} within {} bits]",
                needle, max_allowed_offset
            ),
        ))
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let mut val = self.value;
        loop {
            if val == 0 {
                //return w.write_bit(false);
                return write_bits(w, 0, 1);
            }
            //w.write_bit(true)?;
            write_bits(w, 1, 1)?;
            let uc: u8 = (val & 0xFF) as u8;
            //w.write::<8, _>(uc)?;
            write_bits(w, uc, 8)?;
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
                Ok(UnsignedInteger { value })
            }

            fn visit_u64<E>(self, value: u64) -> Result<UnsignedInteger, E>
            where
                E: serde::de::Error,
            {
                Ok(UnsignedInteger {
                    value: value as u32,
                })
            }
        }
        deserializer.deserialize_u32(U32Visitor)
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
                //let uc8: u8 = rdr.read_to().unwrap();
                let uc8: u8 = UnsignedCharacter::from_reader(rdr)?.value;
                let uc: char = uc8 as char;
                value.push(uc);
            }
        }
        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        if self.value.is_empty() {
            return write_bits(w, 0, 1);
        }
        write_bits(w, 1, 1);
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
        while read_bits(rdr, 1)? != 0 {
            let ival8: u8 = read_bits(rdr, 8)?;
            let ival: i32 = ival8 as i32;
            ii |= ival << 8 * j;
            j += 1;
        }
        if j > 0 {
            ii <<= (4 - j) * 8;
            ii >>= (4 - j) * 8;
        }
        Ok(Self { value: ii })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let mut val = self.value;
        if val == 0 {
            return write_bits(w, 0, 1);
        }
        loop {
            let loc = val & 0xFF;
            //w.write_bit(true)?;
            write_bits(w, 1, 1);
            let uc: u8 = (val & 0xFF) as u8;
            //w.write::<8, _>(uc)?;
            write_bits(w, uc, 8);

            val = val >> 8;
            if (val == 0 && (loc & 0x80) == 0) || (val == -1 && (loc & 0x80) != 0) {
                //return w.write_bit(false);
                return write_bits(w, 0, 1);
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
        let d = double::read_double_from_reader(rdr)?;
        Ok(Self { value: d })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        let rv = double::write_double_to_writer(w, self.value);
        if rv.is_ok() {
            Ok(())
        } else {
            Err(rv.err().unwrap())
        }
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
                Ok(Double {
                    value: value as f64,
                })
            }

            fn visit_i64<E>(self, value: i64) -> Result<Double, E>
            where
                E: serde::de::Error,
            {
                Ok(Double {
                    value: value as f64,
                })
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
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        trace!(
            "{}UserData::from_reader() bp={}",
            indent::get(),
            rdr.position_in_bits()?
        );
        let num_bits: u32 = UnsignedInteger::from_reader(rdr)?.value;
        let mut data: Vec<bool> = Vec::with_capacity(num_bits as usize);
        for _i in 0..num_bits {
            //data.push(rdr.read_bit()?);
            data.push(read_bits(rdr, 1)? != 0);
        }
        Ok(Self { data })
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
    get_number_of_bits_used_to_store_unsigned_integer(u) + 1
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedIntegerWithVariableBitNumber {
    pub value: u32,
}
impl UnsignedIntegerWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(rdr: &mut R, num_bits: u32) -> io::Result<Self> {
        //println!("UnsignedIntegerWithVariableBitNumber: {}", num_bits);
        //assert!(num_bits > 0);
        //assert!(num_bits < 31);
        let mut value = 0u32;
        for u in 0..num_bits {
            //let b: u32 = ((rdr.read_bit()? as u8) & 0x01) as u32;
            let b: u32 = ((read_bits(rdr, 1)? as u8) & 0x01) as u32;
            value |= b << (num_bits - u - 1);
        }
        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, num_bits: u32) -> std::io::Result<()> {
        //assert!(num_bits > 0);
        //assert!(num_bits < 31);
        let mut uval = self.value;
        for u in 0..num_bits {
            let test = 1 << (num_bits - 1 - u);
            if uval >= test {
                //let _ = w.write_bit(true)?;
                let _ = write_bits(w, 1, 1)?;
                uval -= test;
            } else {
                //let _ = w.write_bit(false)?;
                let _ = write_bits(w, 0, 1)?;
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
        //assert!(num_bits >= 1);
        //assert!(num_bits < 31);

        let is_neg = read_bits(r, 1)? != 0;
        let ui;
        if num_bits == 1 {
            ui = 0;
        } else {
            ui = UnsignedIntegerWithVariableBitNumber::from_reader(r, num_bits - 1)?.value;
        }
        let value = if !is_neg { ui as i32 } else { -(ui as i32) };

        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, num_bits: u32) -> std::io::Result<()> {
        //assert!(num_bits > 1);
        //assert!(num_bits < 31);

        //w.write_bit(self.value < 0)?;
        write_bits(w, (self.value < 0) as u8, 1);
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

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub struct CompressedEntityType {
    pub value: u8,
    pub is_a_curve: bool,
}
impl CompressedEntityType {
    pub fn from_reader_and_seek_back<
        R: std::io::Read + std::io::Seek,
        E: bitstream_io::Endianness,
    >(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        let pos = rdr.position_in_bits()?;
        let rv = CompressedEntityType::from_reader(rdr)?;
        rdr.seek_bits(SeekFrom::Start(pos))?;
        assert_eq!(pos, rdr.position_in_bits()?);
        Ok(rv)
    }
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        rdr: &mut BitReader<R, E>,
    ) -> io::Result<Self> {
        trace!(
            "{}CompressedEntityType::from_reader() bp={}",
            indent::get(),
            rdr.position_in_bits()?
        );
        //let is_a_curve = rdr.read_bit()?;
        let is_a_curve = read_bits(rdr, 1)? != 0;
        let typev;
        if is_a_curve {
            let x2 = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 2)?.value;
            match x2 {
                0 => {
                    typev = PrcCompressedCurveType::PRC_HCG_Line as u8;
                }
                1 => {
                    typev = PrcCompressedCurveType::PRC_HCG_Circle as u8;
                }
                2 => {
                    typev = PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve as u8;
                }
                3 => {
                    let x4 = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 2)?.value;
                    match x4 {
                        0 => {
                            typev = PrcCompressedCurveType::PRC_HCG_Ellipse as u8;
                        }
                        1 => {
                            typev = PrcCompressedCurveType::PRC_HCG_CompositeCurve as u8;
                        }
                        _ => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!(
                                    "CompressedEntityType: unknown 4-bit curve pattern ({})! bp={}",
                                    x2 * 4 + x4,
                                    rdr.position_in_bits()?
                                ),
                            ));
                        }
                    }
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "CompressedEntityType: unknown 2-bit curve pattern ({})! bp={}",
                            x2,
                            rdr.position_in_bits()?
                        ),
                    ));
                }
            };
        } else {
            typev = UnsignedIntegerWithVariableBitNumber::from_reader(rdr, 4)?.value as u8;
            let _trial = PrcCompressedFaceType::try_from(typev);
            if !_trial.is_ok() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "CompressedEntityType: unknown face pattern: ({})! bp={}",
                        typev,
                        rdr.position_in_bits()?
                    ),
                ));
            }
        }
        //dbg!(&e);
        let rv = CompressedEntityType {
            value: typev,
            is_a_curve,
        };
        //dbg!(rv);
        Ok(rv)
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W) -> std::io::Result<()> {
        //w.write_bit(self.is_a_curve)?;
        write_bits(w, self.is_a_curve as u8, 1);
        if self.is_a_curve {
            match self.value {
                val if val == PrcCompressedCurveType::PRC_HCG_Line as u8
                    || val == PrcCompressedCurveType::PRC_HCG_Circle as u8
                    || val == PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve as u8 =>
                {
                    UnsignedIntegerWithVariableBitNumber {
                        value: self.value as u32,
                    }
                    .to_writer(w, 2)?;
                }
                val if val == PrcCompressedCurveType::PRC_HCG_Ellipse as u8
                    || val == PrcCompressedCurveType::PRC_HCG_CompositeCurve as u8 =>
                {
                    UnsignedIntegerWithVariableBitNumber {
                        value: self.value as u32,
                    }
                    .to_writer(w, 4)?;
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "CompressedEntityType: unknown curve pattern: ({})!",
                            self.value
                        ),
                    ));
                }
            }
        } else {
            let _trial = PrcCompressedFaceType::try_from(self.value);
            if !_trial.is_ok() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "CompressedEntityType: unknown face pattern: ({})!",
                        self.value
                    ),
                ));
            }
            UnsignedIntegerWithVariableBitNumber {
                value: self.value as u32,
            }
            .to_writer(w, 4)?;
        }
        Ok(())
    }
    #[allow(non_snake_case)]
    pub fn is_PRC_HCG_NewLoop(&self) -> bool {
        !self.is_a_curve && self.value == PrcCompressedFaceType::PRC_HCG_NewLoop as u8
    }
    #[allow(non_snake_case)]
    pub fn is_PRC_HCG_EndLoop(&self) -> bool {
        !self.is_a_curve && self.value == PrcCompressedFaceType::PRC_HCG_EndLoop as u8
    }
}
impl fmt::Debug for CompressedEntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.is_a_curve {
            let e = PrcCompressedFaceType::try_from(self.value).unwrap();
            write!(
                f,
                "CompressedEntityType(value: {} ({}), is_a_curve: {})",
                e, e as u32, self.is_a_curve,
            )
        } else {
            let e = PrcCompressedCurveType::try_from(self.value).unwrap();
            write!(
                f,
                "CompressedEntityType(value: {} ({}), is_a_curve: {})",
                e, e as u32, self.is_a_curve,
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
        let is_compressed_dv = true;
        let sign_extend = false;
        let a = crate::huffman::read_huffman_to_element_array_i8(
            r,
            has_is_compressed_bit,
            num_bits_per_elem,
            is_compressed_dv,
            sign_extend,
        )?;

        Ok(Self { a })
    }
    pub fn from_reader2<R: BitRead>(
        r: &mut R,
        has_is_compressed_bit: bool,
        num_bits_per_elem: u8,
        is_compressed_dv: bool,
        sign_extend: bool,
    ) -> io::Result<Self> {
        let a = crate::huffman::read_huffman_to_element_array_i8(
            r,
            has_is_compressed_bit,
            num_bits_per_elem,
            is_compressed_dv,
            sign_extend,
        )?;
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        _num_bits_per_elem: u8,
    ) -> std::io::Result<()> {
        unimplemented!("{}: Not implemented!", function!());
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
        let a = crate::huffman::read_huffman_to_element_array_i16(
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
        unimplemented!("{}: Not implemented!", function!());
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
            CharacterArray::from_reader2(_rdr, has_is_compressed_bit, 6, true, true)?.a;
        let mut a: Vec<i32> = Vec::with_capacity(num_bits_used_to_store_ints.len());
        for i in 0..num_bits_used_to_store_ints.len() {
            let num_bits_in_int = num_bits_used_to_store_ints[i] as u32;
            a.push(IntegerWithVariableBitNumber::from_reader(_rdr, num_bits_in_int)?.value);
        }
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, _w: &mut W) -> std::io::Result<()> {
        unimplemented!("{}: Not implemented!", function!());
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
        let sign_extend = true;
        let diff_num_bits_used_to_store_ints = CharacterArray::from_reader2(
            r,
            has_is_compressed_bit,
            num_bits_used_to_store_chars,
            is_compressed_dv,
            sign_extend,
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
        pi_array.push(IntegerWithVariableBitNumber::from_reader(r, c_bit_count as u32)?.value);
        for i in 1..diff_num_bits_used_to_store_ints.len() {
            pc_array.push(diff_num_bits_used_to_store_ints[i] as i8);

            c_bit_count += pc_array[i];
            let ival = IntegerWithVariableBitNumber::from_reader(r, c_bit_count as u32)?.value;
            let index = ival + pi_array[i - 1];
            assert!(index >= 0);
            pi_array.push(index);
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
        unimplemented!("{}: Not implemented!", function!());
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
        unimplemented!("{}: Not implemented!", function!());
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
    pub value: f64,
}
impl DoubleWithVariableBitNumber {
    pub fn from_reader<R: BitRead>(
        _rdr: &mut R,
        num_bits: u32,
        tolerance: f64,
    ) -> io::Result<Self> {
        assert!(num_bits > 0);
        //assert!(num_bits <= 30); // if greater Double is used in CompressedNurbs
        //assert!(tolerance > 0.0); // 0.0 is also acceptable

        let neg = read_bits(_rdr, 1)? != 0;

        if num_bits == 1 {
            return Ok(Self {
                value: if neg { -0.0 } else { 0.0 },
            });
        }

        let mut u_temp_value = 0;
        for u in 0..(num_bits - 1) {
            let exp = num_bits - 2 - u;
            let thres = 1 << exp;
            let b = read_bits(_rdr, 1)? != 0;
            if b {
                u_temp_value += thres;
            }
        }
        let value = (u_temp_value as f64) * tolerance * (if neg { -1.0 } else { 1.0 });
        Ok(Self { value })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        assert!(num_bits > 0);
        //assert!(num_bits <= 30); // if greater Double is used in CompressedNurbs
        assert!(tolerance > 0.0);

        let _ = write_bits(_w, (self.value < 0.0) as u8, 1)?;
        if num_bits == 1 {
            return Ok(());
        }

        let mut u_temp_value = (self.value.abs() / tolerance) as u32;
        let test = self.value.abs() / tolerance - u_temp_value as f64;
        if test > 0.5 {
            u_temp_value += 1;
        }

        for u in 0..(num_bits - 1) {
            let exp = num_bits - 2 - u;
            let thres = 1 << exp;
            if u_temp_value >= thres {
                write_bits(_w, 1, 1);
                u_temp_value -= thres;
            } else {
                write_bits(_w, 0, 1);
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

/// Only occurrences are within CompressedControlPoints, PRC_HCG_BSplineHermiteCurve
/// and CompressedPoint.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct Point3DWithVariableBitNumber {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Point3DWithVariableBitNumber {
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        _rdr: &mut BitReader<R, E>,
        num_bits: u32,
        tolerance: f64,
    ) -> io::Result<Self> {
        trace!(
            "{}Point3DWithVariableBitNumber::from_reader() bp={}",
            indent::get(),
            _rdr.position_in_bits()?
        );

        let x = DoubleWithVariableBitNumber::from_reader(_rdr, num_bits, tolerance)?.value;
        let y = DoubleWithVariableBitNumber::from_reader(_rdr, num_bits, tolerance)?.value;
        let z = DoubleWithVariableBitNumber::from_reader(_rdr, num_bits, tolerance)?.value;

        Ok(Self { x, y, z })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        // https://github.com/pdf-association/pdf-issues/issues/581 <- OLD, buggy
        // https://github.com/pdf-association/pdf-issues/issues/706
        DoubleWithVariableBitNumber { value: self.x }.to_writer(_w, num_bits, tolerance)?;
        DoubleWithVariableBitNumber { value: self.y }.to_writer(_w, num_bits, tolerance)?;
        DoubleWithVariableBitNumber { value: self.z }.to_writer(_w, num_bits, tolerance)?;
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
}
impl CompressedPoint {
    pub fn from_reader<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        _rdr: &mut BitReader<R, E>,
        tolerance: f64,
    ) -> io::Result<Self> {
        trace!(
            "{}CompressedPoint::from_reader() bp={}",
            indent::get(),
            _rdr.position_in_bits()?
        );
        assert!(tolerance > 0.0);
        let num_bits = UnsignedIntegerWithVariableBitNumber::from_reader(_rdr, 6)?.value;
        let x;
        let y;
        let z;
        if num_bits == 0 {
            x = 0.0;
            y = 0.0;
            z = 0.0;
        } else if num_bits <= 30 {
            let pt = Point3DWithVariableBitNumber::from_reader(_rdr, num_bits, tolerance)?;
            x = pt.x;
            y = pt.y;
            z = pt.z;
        } else {
            x = Double::from_reader(_rdr)?.value;
            y = Double::from_reader(_rdr)?.value;
            z = Double::from_reader(_rdr)?.value;
        }
        Ok(Self { x, y, z })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(
        &self,
        _w: &mut W,
        //_num_bits: u32,
        tolerance: f64,
    ) -> std::io::Result<()> {
        // https://github.com/pdf-association/pdf-issues/issues/581 <- OLD, buggy
        // https://github.com/pdf-association/pdf-issues/issues/706
        assert!(tolerance > 0.0);
        let xi = (self.x / tolerance + 0.5) as i32;
        let yi = (self.y / tolerance + 0.5) as i32;
        let zi = (self.z / tolerance + 0.5) as i32;
        let num_bits = get_number_of_bits_used_to_store_integer(max(xi.abs(), yi.abs(), zi.abs()));
        let _ = UnsignedIntegerWithVariableBitNumber { value: num_bits }.to_writer(_w, 6)?;
        if num_bits == 0 {
        } else if num_bits <= 30 {
            Point3DWithVariableBitNumber {
                x: self.x,
                y: self.y,
                z: self.z,
            }
            .to_writer(_w, num_bits, tolerance)?;
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
            //let b = rdr.read_bit()?;
            let b = read_bits(rdr, 1)? != 0;
            a[u] = b;
        }
        Ok(Self { a })
    }
    pub fn to_writer<W: BitWrite + ?Sized>(&self, w: &mut W, _: u32) -> std::io::Result<()> {
        for u in 0..self.a.len() {
            //let _ = w.write_bit(self.a[u])?;
            write_bits(w, self.a[u] as u8, 1);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_common::tests::*;
    use bitstream_io::{BigEndian, BitWriter, Endianness};
    use std::fs::File;
    use std::io::Read;
    use std::io::{BufRead, Cursor, Write};
    use std::ops::Add;

    #[test]
    fn io_endian() {
        let mut bytes = vec![];

        let value_lo: u16 = 0x0FF0;

        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::LittleEndian);
            w.write::<16, _>(value_lo).unwrap();
        }
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], 0xF0);
        assert_eq!(bytes[1], 0x0F);
        let us = (bytes[1] as u16) << 8 | bytes[0] as u16;
        assert_eq!(us, value_lo);

        {
            let mut w = BitWriter::endian(Cursor::new(&mut bytes), bitstream_io::BigEndian);
            w.write::<16, _>(value_lo).unwrap();
        }
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], 0x0F);
        assert_eq!(bytes[1], 0xF0);
        let us = (bytes[0] as u16) << 8 | bytes[1] as u16;
        assert_eq!(us, value_lo);
    }

    #[test]
    fn io_bool() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            write_bits(&mut w, false as u8, 1).unwrap();
            write_bits(&mut w, true as u8, 1).unwrap();
            write_bits(&mut w, true as u8, 1).unwrap();
            write_bits(&mut w, true as u8, 1).unwrap();
            write_bits(&mut w, false as u8, 1).unwrap();
            write_bits(&mut w, false as u8, 1).unwrap();
            write_bits(&mut w, false as u8, 1).unwrap();
            write_bits(&mut w, true as u8, 1).unwrap();
        }

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
        assert_eq!(bytes, vec![0b1000_1110]);
        assert_eq!(bytes.len(), 1 as usize);

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        assert_eq!(bytes, vec![0b0111_0001]);
        assert_eq!(bytes.len(), 1 as usize);

        fn internal2<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            assert_eq!(0, bytes.len());
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            let mut b: Boolean = Boolean { value: true };
            let _ = b.to_writer(&mut w);
            b = Boolean { value: false };
            let _ = b.to_writer(&mut w).unwrap();

            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
            assert_eq!(1, bytes.len());

            let bytes_ro: &Vec<u8> = bytes;
            let mut reader = BitReader::endian(Cursor::new(bytes_ro), endian);
            let mut b: bool = Boolean::from_reader(&mut reader).unwrap().value;
            assert_eq!(b, true);
            b = Boolean::from_reader(&mut reader).unwrap().value;
            assert_eq!(b, false);
        }

        let mut bytes = vec![];
        internal2(bitstream_io::LittleEndian, &mut bytes);
        assert_eq!(bytes.len(), 1 as usize);
        assert_eq!(bytes, vec![0b0000_0001]);
        //println!("v={:#?}", bytes);

        let mut bytes = vec![];
        bytes.clear();
        internal2(bitstream_io::BigEndian, &mut bytes);
        assert_eq!(bytes.len(), 1 as usize);
        assert_eq!(bytes, vec![0b1000_0000]);
    }

    /// Test to show that read_bits() and write_bits() are endian-independent. As bit ordering is
    /// defined internally.
    #[test]
    fn io_bits_are_endian_independent() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            write_bits(&mut w, 0xFF, 8);
            write_bits(&mut w, 0xF0, 8);

            write_bits(&mut w, 1, 1);
            write_bits(&mut w, 0, 1);
            write_bits(&mut w, 1, 1);
            write_bits(&mut w, 0, 1);
            write_bits(&mut w, 1, 1);
            write_bits(&mut w, 0, 1);
            write_bits(&mut w, 1, 1);
            write_bits(&mut w, 0, 1);

            let mut r = BitReader::endian(Cursor::new(bytes.as_slice()), endian);
            assert_eq!(0xFF, read_bits(&mut r, 8).unwrap() as u8); // endian independent
            assert_eq!(0xF0, read_bits(&mut r, 8).unwrap() as u8); // endian independent
            assert_eq!(0xAA, read_bits(&mut r, 8).unwrap() as u8); // endian independent
        }

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
        assert_eq!(bytes, vec![0xFF, 0x0F, 0x55]); // this is endian dependent

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        assert_eq!(bytes, vec![0xFF, 0xF0, 0xAA]); // this is endian dependent
    }

    #[test]
    fn io_uchar() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::LittleEndian);
            let mut uc = UnsignedCharacter { value: 125u8 };
            let _ = uc.to_writer(&mut w);
            uc.value = 0u8;
            let _ = uc.to_writer(&mut w);
            uc.value = 255u8;
            let _ = uc.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");

            let bytes_ro: &Vec<u8> = bytes;
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
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

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
        assert_eq!(bytes.len(), 3 as usize);

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        assert_eq!(bytes.len(), 3 as usize);
    }

    #[test]
    fn io_ushort() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut us = UnsignedShort {
                value: (3_u8 as u16) << 8 | 1_u8 as u16,
            };
            let _ = us.to_writer(&mut w);
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");

            assert_eq!(bytes.len(), 2_usize);

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
            assert_eq!(
                UnsignedShort::from_reader(&mut r).unwrap().value,
                0b00000011_00000001
            );
        }

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
    }

    #[test]
    fn io_only_uint() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            // {
            //     let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut ui = UnsignedInteger { value: 125 };
            let _ = ui.to_writer(&mut w);
            ui.value = 0;
            let _ = ui.to_writer(&mut w);
            ui.value = 1239255;
            let _ = ui.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
            //}
            assert_eq!(bytes.len(), 5 as usize);

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
            assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 125);
            assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 0);
            assert_eq!(UnsignedInteger::from_reader(&mut r).unwrap().value, 1239255);
        }

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
    }

    #[test]
    fn io_string() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            // {
            //     let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let ss = std::string::String::from(
                "Abracadabra order matters:77 CCCCitStream last to initialized last bla-bla 1234",
            );
            let s = String { value: ss.clone() };
            let semp: String = Default::default();

            semp.to_writer(&mut w).unwrap();
            s.to_writer(&mut w).unwrap();

            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
            // }
            assert_eq!(bytes.len(), 81);

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
            assert_eq!(String::from_reader(&mut r).unwrap().value, "");
            assert_eq!(String::from_reader(&mut r).unwrap().value, ss);
        }

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_only_int() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let mut i = Integer { value: 125 };
            let _ = i.to_writer(&mut w);
            i.value = 0;
            let _ = i.to_writer(&mut w);
            i.value = -1239255;
            let _ = i.to_writer(&mut w);

            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
            //}
            assert_eq!(bytes.len(), 5 as usize);

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
            assert_eq!(Integer::from_reader(&mut r).unwrap().value, 125);
            assert_eq!(Integer::from_reader(&mut r).unwrap().value, 0);
            assert_eq!(Integer::from_reader(&mut r).unwrap().value, -1239255);
        }

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);

        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn read_ints() {
        fn reverse_bits(_x: u32) -> u32 {
            let mut x = _x;
            x = (((x & 0xaaaaaaaa) >> 1) | ((x & 0x55555555) << 1));
            x = (((x & 0xcccccccc) >> 2) | ((x & 0x33333333) << 2));
            x = (((x & 0xf0f0f0f0) >> 4) | ((x & 0x0f0f0f0f) << 4));
            x = (((x & 0xff00ff00) >> 8) | ((x & 0x00ff00ff) << 8));
            return ((x >> 16) | (x << 16));
        }

        let path = std::env::current_dir().unwrap();
        println!("The current directory is {}", path.display());
        let bytes_external =
            get_file_as_byte_vec(&std::string::String::from("testdata/read_ints.bin"));
        assert_eq!(bytes_external.len(), 808992 as usize);

        let n: u32 = 66002;
        let mut r = BitReader::endian(Cursor::new(&bytes_external), bitstream_io::BigEndian);
        for i in 0..n {
            let u1 = UnsignedInteger::from_reader(&mut r).unwrap().value;
            let i1 = Integer::from_reader(&mut r).unwrap().value;
            let i2 = Integer::from_reader(&mut r).unwrap().value;
            let u2 = r.read::<32, u32>().unwrap().swap_bytes();

            assert_eq!(i, u1);
            assert_eq!(i, i1 as u32);
            assert_eq!(i, -i2 as u32);
            //println!("{:b} {:b}", i, u2);
            assert_eq!(i, u2);
        }

        let mut bytes = vec![];
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = UnsignedInteger { value: i }.to_writer(&mut w).unwrap();
                let _ = Integer { value: i as i32 }.to_writer(&mut w).unwrap();
                let _ = Integer { value: -(i as i32) }.to_writer(&mut w).unwrap();
                w.write_unsigned::<32, u32>((i as u32).swap_bytes())
                    .unwrap();
            }
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes_external, bytes);
    }

    #[test]
    fn io_nbb_uint_vbr() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);

            let n: u32 = 31966002;

            let trailing_bits: u64;
            //{
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let _ = NumberOfBitsThenUnsignedInteger { value: i }
                    .to_writer(&mut w)
                    .unwrap();
            }
            trailing_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end") as u64;
            //}
            assert_eq!(115678204, bytes.len());

            let mut r = BitReader::endian(Cursor::new(&bytes), endian);
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

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_uint_vbr() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);

            let n: u32 = 31966002;

            let trailing_bits: u64;
            //{
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in 0..n {
                let nb = get_number_of_bits_used_to_store_unsigned_integer(i);
                let _ = UnsignedIntegerWithVariableBitNumber { value: i }
                    .to_writer(&mut w, nb)
                    .unwrap();
            }
            trailing_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end") as u64;
            //}
            assert_eq!(95699453, bytes.len());

            let mut r = BitReader::endian(Cursor::new(&bytes), endian);
            for i in 0..n {
                let nb = get_number_of_bits_used_to_store_unsigned_integer(i);
                let u1 = UnsignedIntegerWithVariableBitNumber::from_reader(&mut r, nb)
                    .unwrap()
                    .value;

                assert_eq!(i, u1);
            }
            assert_eq!(
                95699453 as u64 * 8,
                r.position_in_bits().unwrap() + trailing_bits
            );
        }

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_int_vbr() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);

            let n: i32 = 1966002;

            let trailing_bits: u64;
            //{
            //let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            for i in -n..n {
                let nb = get_number_of_bits_used_to_store_integer(i);
                let _ = IntegerWithVariableBitNumber { value: i }
                    .to_writer(&mut w, nb)
                    .unwrap();
            }
            trailing_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end") as u64;
            //}
            assert_eq!(10288726, bytes.len());

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
            for i in -n..n {
                let nb = get_number_of_bits_used_to_store_integer(i);
                let u1 = IntegerWithVariableBitNumber::from_reader(&mut r, nb)
                    .unwrap()
                    .value;

                assert_eq!(i, u1);
            }
            assert_eq!(
                10288726 as u64 * 8,
                r.position_in_bits().unwrap() + trailing_bits
            );
        }

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_compressed_entity_type() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);

            let c_line = CompressedEntityType {
                value: PrcCompressedCurveType::PRC_HCG_Line as u8,
                is_a_curve: true,
            };

            c_line.to_writer(&mut w).unwrap();
            CompressedEntityType {
                value: PrcCompressedCurveType::PRC_HCG_Circle as u8,
                is_a_curve: true,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve as u8,
                is_a_curve: true,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedCurveType::PRC_HCG_Ellipse as u8,
                is_a_curve: true,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedCurveType::PRC_HCG_CompositeCurve as u8,
                is_a_curve: true,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_NewLoop as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_EndLoop as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoPlane as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();
            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoCylinder as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoTorus as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoSphere as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoCone as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_IsoNurbs as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaPlane as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaCylinder as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaTorus as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaSphere as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaCone as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaNurbs as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            CompressedEntityType {
                value: PrcCompressedFaceType::PRC_HCG_AnaGenericFace as u8,
                is_a_curve: false,
            }
            .to_writer(&mut w)
            .unwrap();

            let trailing_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end")
                as u64;
            assert_eq!(
                3 * 3 + 2 * 5 + 15 * 5 + trailing_bits,
                8 * bytes.len() as u64
            );

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);

            assert_eq!(
                PrcCompressedCurveType::try_from(
                    CompressedEntityType::from_reader(&mut r).unwrap()
                )
                .unwrap(),
                PrcCompressedCurveType::PRC_HCG_Line
            );
            assert_eq!(
                PrcCompressedCurveType::try_from(
                    CompressedEntityType::from_reader(&mut r).unwrap()
                )
                .unwrap(),
                PrcCompressedCurveType::PRC_HCG_Circle
            );
            assert_eq!(
                PrcCompressedCurveType::try_from(
                    CompressedEntityType::from_reader(&mut r).unwrap()
                )
                .unwrap(),
                PrcCompressedCurveType::PRC_HCG_BSplineHermiteCurve
            );
            assert_eq!(
                PrcCompressedCurveType::try_from(
                    CompressedEntityType::from_reader(&mut r).unwrap()
                )
                .unwrap(),
                PrcCompressedCurveType::PRC_HCG_Ellipse
            );
            assert_eq!(
                PrcCompressedCurveType::try_from(
                    CompressedEntityType::from_reader(&mut r).unwrap()
                )
                .unwrap(),
                PrcCompressedCurveType::PRC_HCG_CompositeCurve
            );

            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_NewLoop
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_EndLoop
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoPlane
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoCylinder
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoTorus
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoSphere
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoCone
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_IsoNurbs
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaPlane
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaCylinder
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaTorus
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaSphere
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaCone
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaNurbs
            );
            assert_eq!(
                PrcCompressedFaceType::try_from(CompressedEntityType::from_reader(&mut r).unwrap())
                    .unwrap(),
                PrcCompressedFaceType::PRC_HCG_AnaGenericFace
            );
            assert_eq!(
                r.position_in_bits().unwrap(),
                8 * bytes.len() as u64 - trailing_bits
            );
        }

        let mut bytes = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
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
            trailing_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end");
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
            Double { value: 0.1 }.to_writer(&mut w).unwrap();
            Double { value: 0.01 }.to_writer(&mut w).unwrap();
            Double { value: 0.001 }.to_writer(&mut w).unwrap();
            for i in 0..n {
                let u = UnsignedInteger { value: i };
                let d1 = Double {
                    value: i as f64 * 1.15,
                };
                let d2 = Double {
                    value: i as f64 * -1.11,
                };

                u.to_writer(&mut w).unwrap();
                d1.to_writer(&mut w).unwrap();
                d2.to_writer(&mut w).unwrap();

                s.push(serde_json::to_string(&u).unwrap());
                s.push(serde_json::to_string(&d1).unwrap());
                s.push(serde_json::to_string(&d2).unwrap());
                // s = s + &serde_json::to_string(&u).unwrap();
                // s = s + &serde_json::to_string(&d1).unwrap();
                // s = s + &serde_json::to_string(&d2).unwrap();
            }
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }

        println!("bytes: {}", bytes.len());
        assert_eq!(bytes.len(), 95353 as usize);

        //assert_eq!(s.len(), 177612usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(0.1, Double::from_reader(&mut r).unwrap().value);
        assert_eq!(0.01, Double::from_reader(&mut r).unwrap().value);
        assert_eq!(0.001, Double::from_reader(&mut r).unwrap().value);
        for i in 0..n {
            let ui = UnsignedInteger::from_reader(&mut r).unwrap().value;
            assert_eq!(i, ui);
            let ui: u32 = serde_json::from_str(&s[i as usize * 3 + 0]).unwrap();
            assert_eq!(i, ui);

            let mut reference = i as f64 * 1.15;
            let recovered = Double::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
            let recovered: Double = serde_json::from_str(&s[i as usize * 3 + 1]).unwrap();
            assert!((reference - recovered.value).abs() < 1E-9);

            reference = i as f64 * -1.11;
            let recovered = Double::from_reader(&mut r).unwrap().value;
            assert_eq!(reference, recovered);
            let recovered: Double = serde_json::from_str(&s[i as usize * 3 + 2]).unwrap();
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
            FloatAsBytes { value: 0.1 }.to_writer(&mut w).unwrap();
            FloatAsBytes { value: 0.01 }.to_writer(&mut w).unwrap();
            FloatAsBytes { value: 0.001 }.to_writer(&mut w).unwrap();
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
            num_trailing_padding_bits = fill_partial_byte_at_end(&mut w, false)
                .expect("failed to fill partial byte at end");
        }

        println!("bytes: {}", bytes.len());
        assert_eq!(bytes.len(), 685018 as usize);
        assert_eq!(bytes[bytes.len() - 1 - 0], 142);
        assert_eq!(bytes[bytes.len() - 1 - 1], 31);
        assert_eq!(bytes[bytes.len() - 1 - 2], 45);
        assert_eq!(bytes[bytes.len() - 1 - 3], 28);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        assert_eq!(0.1, FloatAsBytes::from_reader(&mut r).unwrap().value);
        assert_eq!(0.01, FloatAsBytes::from_reader(&mut r).unwrap().value);
        assert_eq!(0.001, FloatAsBytes::from_reader(&mut r).unwrap().value);
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
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
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
    #[test]
    #[ignore = "slow"]
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
                let num_trailing_passing_bits = fill_partial_byte_at_end(&mut w, false)
                    .expect("failed to fill partial byte at end");
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
                let num_trailing_passing_bits = fill_partial_byte_at_end(&mut w, false)
                    .expect("failed to fill partial byte at end");
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
    #[test]
    #[ignore = "slow"]
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
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
        }
        assert_eq!(bytes.len(), 17usize);

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let ud = UserData::from_reader(&mut r).unwrap();
        assert_eq!(123usize, ud.data.len());
        assert_eq!(reference, ud);
    }

    #[test]
    fn io_compressed_types() {
        // UnsignedIntegerWithVariableBitNumber
        // DoubleWithVariableBitNumber
        // Point3DWithVariableBitNumber

        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let n = 1000;
            let num_bits = 30;
            let tol = 0.01;

            let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);

            for i in 0..n {
                let _ = UnsignedIntegerWithVariableBitNumber { value: i }
                    .to_writer(&mut w, num_bits)
                    .unwrap();
                let _ = DoubleWithVariableBitNumber {
                    value: i as f64 * -1.11,
                }
                .to_writer(&mut w, num_bits, tol)
                .unwrap();
                let _ = CompressedPoint {
                    x: i as f64 * -1.12,
                    y: i as f64 * 0.97,
                    z: i as f64 * 2.54,
                }
                .to_writer(&mut w, tol)
                .unwrap();
                Point3DWithVariableBitNumber {
                    x: i as f64,
                    y: i as f64 * 2.0,
                    z: i as f64 * -1.5,
                }
                .to_writer(&mut w, num_bits, tol)
                .unwrap();
            }
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
            //}
            //dbg!(bytes.len());
            assert_eq!(26233, bytes.len());
            //assert_eq!(bytes_external, bytes);

            let bytes_ro = bytes.as_slice();
            let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
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

                let p3 = CompressedPoint::from_reader(&mut r, tol).unwrap();
                //dbg!(p3.x - i as f64 * -1.12, tol);
                assert!((p3.x - i as f64 * -1.12).abs() < tol);
                assert!((p3.y - i as f64 * 0.97).abs() < tol);
                assert!((p3.z - i as f64 * 2.54).abs() < tol);

                let pt = Point3DWithVariableBitNumber::from_reader(&mut r, num_bits, tol).unwrap();
                assert!((pt.x - i as f64).abs() < tol);
                assert!((pt.y - i as f64 * 2.0).abs() < tol);
                assert!((pt.z - i as f64 * -1.5).abs() < tol);
            }
        }

        let mut bytes: Vec<u8> = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes: Vec<u8> = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_compressed_fp() {
        fn internal<E: bitstream_io::Endianness + ?Sized + Copy>(endian: E, bytes: &mut Vec<u8>) {
            let paddings = [true, false];
            let nbs: [u32; _] = [25, 29, 30];
            let tols = [1.0, 0.1, 0.01, 0.001];
            let n = 100_i32;

            for p in paddings.iter() {
                let mut w = BitWriter::endian(Cursor::new(&mut *bytes), endian);
                for nb in nbs.iter() {
                    for t in tols.iter() {
                        for i in -n..n {
                            let d = DoubleWithVariableBitNumber {
                                value: i as f64 * 131.785,
                            };

                            d.to_writer(&mut w, *nb, *t).unwrap();
                        }
                    }
                }
                fill_partial_byte_at_end(&mut w, *p).expect("failed to fill partial byte at end");

                let bytes_ro = bytes.as_slice();
                let mut r = BitReader::endian(Cursor::new(&bytes_ro), endian);
                for nb in nbs.iter() {
                    for t in tols.iter() {
                        for i in -n..n {
                            let d = DoubleWithVariableBitNumber::from_reader(&mut r, *nb, *t)
                                .unwrap()
                                .value;
                            let d_ref = i as f64 * 131.785;
                            let test = (d - d_ref).abs();
                            assert!(test < *t);
                        }
                    }
                }
            }
        }

        let mut bytes: Vec<u8> = vec![];
        internal(bitstream_io::BigEndian, &mut bytes);
        let mut bytes: Vec<u8> = vec![];
        internal(bitstream_io::LittleEndian, &mut bytes);
    }

    #[test]
    fn io_uncompressed_arrays() {
        let mut bytes: Vec<u8> = vec![];

        let bools = vec![
            true, false, true, true, false, false, false, false, true, false, true, true, true,
        ];
        let n: usize = 13;
        assert_eq!(bools.len(), n);

        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            let _ = UncompressedBoolArray { a: bools.clone() }
                .to_writer(&mut w, 0)
                .unwrap();
            fill_partial_byte_at_end(&mut w, false).expect("failed to fill partial byte at end");
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
