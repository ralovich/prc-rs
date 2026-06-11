// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use crate::builtin::read_bits;
use crate::builtin::{UnsignedCharacter, UnsignedInteger};
use crate::function;
use bitstream_io::{BitRead, BitReader, LittleEndian};
use log::{debug, warn};
use measure_time::debug_time;
use std::io::{Cursor, SeekFrom};
use std::{fmt, io};

#[allow(unused)]
fn byte_reverse(mut b: u8) -> u8 {
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    b
}

///
///
/// * sign_extend: do not mask leaf values with (2^num_bits_per_elem-1), allows sign extending and returning potentially negative values
pub fn read_huffman_to_element_array_i8<R: BitRead>(
    r: &mut R,
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
    sign_extend: bool,
) -> io::Result<Vec<i8>> {
    //dbg!(has_is_compressed_bit, num_bits_per_elem, is_compressed_dv);
    let mut is_compressed = is_compressed_dv;
    if has_is_compressed_bit {
        //is_compressed = r.read_bit()?;
        is_compressed = read_bits(r, 1)? != 0;
    }
    debug!("is_compressed: {}", is_compressed);

    if !is_compressed {
        let arr_size = UnsignedInteger::from_reader(r)?.value;
        //dbg!(arr_size);
        let mut v = Vec::with_capacity(arr_size as usize);
        for _i in 0..arr_size {
            let val = UnsignedCharacter::from_reader(r)?.value;
            v.push(val as i8);
        }
        assert_eq!(v.len(), arr_size as usize);
        return Ok(v);
    }

    let huffman_array_size = UnsignedInteger::from_reader(r)?.value;
    //dbg!(huffman_array_size);
    if huffman_array_size == 0 {
        return Ok(vec![]);
    }
    let mut huffman_bytes: Vec<u8> = Vec::with_capacity(huffman_array_size as usize * 4);
    for _i in 0..huffman_array_size {
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    assert!(number_of_bits_used_in_last_integer < 32);

    let tot_bits = huffman_bytes.len() * 8 - 32 + number_of_bits_used_in_last_integer as usize;
    //dbg!(tot_bits);

    let v = huffman_decode_i8(huffman_bytes, tot_bits, num_bits_per_elem, sign_extend);
    Ok(v)
}

pub fn read_huffman_to_element_array_i16<R: BitRead>(
    r: &mut R,
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
) -> io::Result<Vec<i16>> {
    //dbg!(has_is_compressed_bit, num_bits_per_elem, is_compressed_dv);
    let mut is_compressed = is_compressed_dv;
    if has_is_compressed_bit {
        //is_compressed = r.read_bit()?;
        is_compressed = read_bits(r, 1)? != 0;
    }
    //dbg!(is_compressed);

    if !is_compressed {
        let arr_size = UnsignedInteger::from_reader(r)?.value;
        //dbg!(arr_size);
        let mut v = Vec::with_capacity(arr_size as usize);
        for _i in 0..arr_size {
            let lo = UnsignedCharacter::from_reader(r)?.value as u16;
            let hi = UnsignedCharacter::from_reader(r)?.value as u16;
            v.push((hi << 8 | lo) as i16);
        }
        assert_eq!(v.len(), arr_size as usize);
        return Ok(v);
    }

    let huffman_array_size = UnsignedInteger::from_reader(r)?.value;
    //dbg!(huffman_array_size);
    if huffman_array_size == 0 {
        return Ok(vec![]);
    }
    let mut huffman_bytes: Vec<u8> = Vec::with_capacity(huffman_array_size as usize * 4);
    for _i in 0..huffman_array_size {
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
        huffman_bytes.push(read_bits(r, 8)?);
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    assert!(number_of_bits_used_in_last_integer < 32);

    let tot_bits = huffman_bytes.len() * 8 - 32 + number_of_bits_used_in_last_integer as usize;
    //dbg!(tot_bits);

    let v = huffman_decode_i16(huffman_bytes, tot_bits, num_bits_per_elem);
    Ok(v)
}

fn bits_equal(lhs: u32, rhs: u32, num_bits: u8) -> bool {
    let mut all_bits_equal = true;
    for k in 0..num_bits {
        all_bits_equal = all_bits_equal && ((lhs >> k) & 0x01 == (rhs >> k) & 0x01);
    }
    all_bits_equal
}

fn rev_bits(val: u32, num_bits: u8) -> u32 {
    let mut rv: u32 = 0;
    for k in 0..num_bits {
        rv = rv << 1;
        let bit = val >> k & 0x01;
        rv = rv | bit;
    }
    rv
}

pub fn huffman_decode_i8(
    prc_huffman_bytes: Vec<u8>,
    _tot_bits: usize,
    num_bits_per_elem: u8,
    sign_extend: bool,
) -> Vec<i8> {
    debug_time!("huffman_decode_i8");
    assert!(num_bits_per_elem <= 8);
    struct HuffTreeLeaf {
        pub value: i8,
        pub code_length: u32,
        pub code_value: u32,
    }
    impl fmt::Debug for HuffTreeLeaf {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{:5} {:8} {:016b}",
                self.value, self.code_length, self.code_value
            )
        }
    }

    let mut r = BitReader::endian(Cursor::new(&prc_huffman_bytes), LittleEndian);
    let num_leaves = r.read_var::<u32>(num_bits_per_elem as u32 + 1).unwrap();
    //dbg!(num_leaves);
    if num_leaves == 0 {
        return vec![];
    }

    let max_code_length = r.read_var::<u8>(8).unwrap();
    //dbg!(max_code_length);

    let mut min_code_bits: u32 = 0;
    let mut max_code_bits: u32 = 0;
    let mut leaves: Vec<HuffTreeLeaf> = Vec::with_capacity(num_leaves as usize);
    for i in 0..num_leaves {
        let value: i8;
        if sign_extend {
            value = r.read_var::<i8>(num_bits_per_elem as u32).unwrap();
        } else {
            let mask = ((1u32 << num_bits_per_elem) - 1) as u8;
            assert_ne!(mask, 0);
            let val = r.read_var::<i8>(num_bits_per_elem as u32).unwrap();
            value = (val as u8 & mask) as i8;
        }
        let code_length = r.read_var::<u32>(max_code_length as u32).unwrap();
        let code_value = r.read_var::<u32>(code_length).unwrap();
        if i == 0 {
            min_code_bits = code_length;
            max_code_bits = code_length;
        } else {
            min_code_bits = std::cmp::min(min_code_bits, code_length);
            max_code_bits = std::cmp::max(max_code_bits, code_length);
        }
        leaves.push(HuffTreeLeaf {
            value,
            code_length,
            code_value,
        });
    }
    //dbg!(&leaves.len(), min_code_bits, max_code_bits);
    leaves.sort_by(|a, b| a.code_value.cmp(&b.code_value));
    //dbg!(&leaves);

    let elem_array_size = r.read::<32, u32>().unwrap();
    //dbg!(elem_array_size);
    let mut data: Vec<i8> = Vec::with_capacity(elem_array_size as usize);
    for _i in 0..elem_array_size {
        let mut selected_leaf: Option<usize> = None;
        for nb in min_code_bits..=max_code_bits {
            let before = r.position_in_bits().unwrap();
            let trial: u32 = r.read_var::<u32>(nb).unwrap();
            for j in 0..leaves.len() {
                if nb != leaves[j].code_length {
                    continue;
                }
                //debug!("Trialling (for elem #{}) @ {} bits: {:b} {:b} ", _i, nb, trial, leaves[j].code_value);
                if bits_equal(trial, rev_bits(leaves[j].code_value, nb as u8), nb as u8) {
                    selected_leaf = Some(j);
                    break;
                }
                // if selected_leaf != None {
                //     break;
                // }
            }
            if selected_leaf == None {
                let _ = r.seek_bits(SeekFrom::Start(before));
            } else {
                break;
            }
        }
        if selected_leaf == None {
            warn!("Could not decode value for element {}", _i);
            return vec![];
        }

        data.push(leaves[selected_leaf.unwrap() as usize].value);
    }
    assert_eq!(data.len(), elem_array_size as usize);
    if _tot_bits as u64 != r.position_in_bits().unwrap() {
        debug!(
            "{}: {} bits consumed of {} bits",
            function!(),
            r.position_in_bits().unwrap(),
            _tot_bits
        );
    }

    data
}

pub fn huffman_decode_i16(
    prc_huffman_bytes: Vec<u8>,
    _tot_bits: usize,
    num_bits_per_elem: u8,
) -> Vec<i16> {
    debug_time!("huffman_decode_i16");
    struct HuffTreeLeaf {
        pub value: i16,
        pub code_length: u32,
        pub code_value: u32,
    }
    impl fmt::Debug for HuffTreeLeaf {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{:7} {:8} {:016b}",
                self.value, self.code_length, self.code_value
            )
        }
    }

    let mut r = BitReader::endian(Cursor::new(&prc_huffman_bytes), LittleEndian);
    let num_leaves = r.read_var::<u32>(num_bits_per_elem as u32 + 1).unwrap();
    //dbg!(num_leaves);
    if num_leaves == 0 {
        return vec![];
    }

    let max_code_length = r.read_var::<u8>(8).unwrap();
    //dbg!(max_code_length);

    let mut min_code_bits: u32 = 0;
    let mut max_code_bits: u32 = 0;
    let mut leaves: Vec<HuffTreeLeaf> = Vec::with_capacity(num_leaves as usize);
    for i in 0..num_leaves {
        // FIXME: is sign extension needed?
        let value = r.read_var::<i16>(num_bits_per_elem as u32).unwrap();
        let code_length = r.read_var::<u32>(max_code_length as u32).unwrap();
        let code_value = r.read_var::<u32>(code_length).unwrap();
        if i == 0 {
            min_code_bits = code_length;
            max_code_bits = code_length;
        } else {
            min_code_bits = std::cmp::min(min_code_bits, code_length);
            max_code_bits = std::cmp::max(max_code_bits, code_length);
        }
        leaves.push(HuffTreeLeaf {
            value,
            code_length,
            code_value,
        });
    }
    //dbg!(&leaves.len(), min_code_bits, max_code_bits);
    leaves.sort_by(|a, b| a.code_value.cmp(&b.code_value));
    //dbg!(&leaves);

    let elem_array_size = r.read::<32, u32>().unwrap();
    //dbg!(elem_array_size);
    let mut data: Vec<i16> = Vec::with_capacity(elem_array_size as usize);
    for _i in 0..elem_array_size {
        let mut selected_leaf: Option<usize> = None;
        for nb in min_code_bits..=max_code_bits {
            let before = r.position_in_bits().unwrap();
            let trial: u32 = r.read_var::<u32>(nb).unwrap();
            for j in 0..leaves.len() {
                if nb != leaves[j].code_length {
                    continue;
                }
                //debug!("Trialling (for elem #{}) @ {} bits: {:b} {:b} ", _i, nb, trial, leaves[j].code_value);
                if bits_equal(trial, rev_bits(leaves[j].code_value, nb as u8), nb as u8) {
                    selected_leaf = Some(j);
                    break;
                }
                // if selected_leaf != None {
                //     break;
                // }
            }
            if selected_leaf == None {
                let _ = r.seek_bits(SeekFrom::Start(before));
            } else {
                break;
            }
        }
        if selected_leaf == None {
            warn!("Could not decode value for element {}", _i);
            return vec![];
        }

        data.push(leaves[selected_leaf.unwrap() as usize].value);
    }
    assert_eq!(data.len(), elem_array_size as usize);
    if _tot_bits as u64 != r.position_in_bits().unwrap() {
        debug!(
            "{}: {} bits consumed of {} bits",
            function!(),
            r.position_in_bits().unwrap(),
            _tot_bits
        );
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_packing() {
        assert_eq!(0x00, byte_reverse(0x00));
        assert_eq!(0x01, byte_reverse(0x80));
        assert_eq!(0x08, byte_reverse(0x10));
        assert_eq!(0x0F, byte_reverse(0xF0));
        assert_eq!(0xF0, byte_reverse(0x0F));
        assert_eq!(0xFF, byte_reverse(0xFF));
    }

    #[test]
    fn test_bits_equal() {
        assert_ne!(0b1010, 0b0010);
        assert!(bits_equal(0b1010, 0b0010, 2));
    }

    #[test]
    fn test_rev_bits() {
        assert_eq!(0b1000, rev_bits(0b0001, 4));
        assert_eq!(0b0001, rev_bits(0b1000, 4));

        assert_eq!(0b0100, rev_bits(0b0001, 3));

        assert_eq!(0b0000, rev_bits(0b0000, 5));
    }

    #[test]
    fn test_huffman_decode() {
        let num_bits = 6u8;
        // 3D-PDF-Sample-Aero-Composite-Part.stream-48 point_array in File Structure 0: section TESSELLATION_SECTION: 65438 bytes
        let number_of_bits_used_in_last_integer = 15usize;
        #[rustfmt::skip]
        let huffman_array : [u32; 288] = [
            4236313232,
            1133090432,
            2557086766,
            2492145908,
            1213468830,
            2234713391,
            607823159,
            2839753503,
            2066257010,
            1275068446,
            846543517,
            748632809,
            4045375694,
            3755933411,
            1384101883,
            4276059262,
            2415327794,
            3103712251,
            4232569407,
            858574728,
            3432225483,
            1570364334,
            4175358967,
            2900950670,
            3622067513,
            4181642494,
            3606526837,
            3191323389,
            995741435,
            3673983923,
            1993339519,
            1975373294,
            3185045183,
            1664597489,
            1799321022,
            2932669599,
            1663510525,
            3739741883,
            3958856668,
            4150119550,
            3326539718,
            1975411580,
            686288575,
            3613816051,
            3147011070,
            1869839261,
            3116223982,
            4258755359,
            3338100666,
            4222388471,
            1573680945,
            3430870007,
            3907514039,
            668970700,
            4147864359,
            3454955379,
            4221295869,
            1979424697,
            3185045183,
            1663188465,
            4147878126,
            2950491747,
            3705614843,
            3392781775,
            3562618572,
            4024581427,
            2134896119,
            2645789490,
            4000281467,
            934919629,
            3864839927,
            2129129086,
            467494348,
            574607839,
            999878243,
            1673105126,
            3184516334,
            3891164977,
            686288510,
            1869027015,
            3657385852,
            4140807803,
            2623225561,
            2313491437,
            4149308642,
            2627509406,
            2631179476,
            2649369812,
            1064455908,
            3907898302,
            3376086671,
            1977606074,
            1556606651,
            1556503750,
            1708064486,
            2388320238,
            2078126573,
            4159991675,
            4219320694,
            4266917327,
            1028602841,
            2812083829,
            4204592359,
            3819871863,
            1743732659,
            2135410300,
            3732694982,
            1283063503,
            1064560591,
            4148130807,
            4258156103,
            2683763250,
            332020157,
            4070567551,
            2396830691,
            1806138110,
            1925853772,
            1022686319,
            3330172149,
            3620109415,
            1180564472,
            3482123999,
            3908013304,
            2681125881,
            4198481891,
            1869799367,
            3120359412,
            389807005,
            3124559449,
            1574113606,
            3120129723,
            4149834525,
            4058402487,
            3623272057,
            4131369981,
            1199341263,
            3919343487,
            2127553341,
            1945591581,
            3444569791,
            3573348841,
            3715625971,
            2129361839,
            3367624686,
            4220763384,
            4248688221,
            3366141817,
            573465133,
            4135152507,
            4259794615,
            3892019163,
            3929761710,
            2067782132,
            2079800099,
            2745660206,
            2647439167,
            2138430660,
            3086383467,
            4112997142,
            2932862462,
            1038941031,
            3135164283,
            779615135,
            1672953763,
            3048982766,
            852950735,
            4160125290,
            4059813708,
            4140261277,
            3489037273,
            3975656414,
            1194441869,
            2302385612,
            1061153007,
            3755835111,
            1803511770,
            2499278457,
            3802066787,
            762824644,
            4213102827,
            1054109539,
            3017723771,
            3729255671,
            2750882712,
            4221355135,
            2674540405,
            3038753789,
            3755605711,
            3967447507,
            1161690511,
            3354851101,
            2065619198,
            1068621511,
            1740457718,
            4215265775,
            2396286823,
            3997643196,
            965279559,
            3657362423,
            2137512591,
            954855275,
            4254000925,
            3354163121,
            2986183833,
            4285522557,
            1677401992,
            3639929979,
            3737646651,
            3354804145,
            3975656414,
            1870003005,
            2043924366,
            2671582842,
            1939072859,
            1739110871,
            1068744506,
            3954824447,
            4193044796,
            2744982748,
            3551780083,
            2381019985,
            3967771603,
            1027145103,
            2354780342,
            3967787565,
            3737615133,
            463226289,
            1673367271,
            3181374719,
            2337873511,
            3454420378,
            2361128380,
            4170184434,
            4160208829,
            3394237566,
            997388175,
            3103748090,
            3103712211,
            2120999487,
            1505545702,
            1330735931,
            2000551311,
            3518613484,
            1938339391,
            2008145607,
            1736238028,
            3324560941,
            2624483769,
            660864283,
            770473827,
            4089328103,
            1943969396,
            4025015543,
            3409949241,
            4257735219,
            4025450353,
            2493507837,
            4142259999,
            1912508397,
            4146060415,
            529071175,
            2811337201,
            511179544,
            803798263,
            1929285582,
            4270844107,
            3711769566,
            3111929575,
            4288929553,
            1693348164,
            2398682298,
            4155440638,
            2683600619,
            419359522,
            2117591615,
            4026386415,
            1869803407,
            1910501364,
            2298234791,
            2078798143,
            3187535812,
            3215194047,
            2751151912,
            4277649279,
            2414755768,
            4217347064,
            1071376587,
            106085,
        ];
        assert_eq!(288, huffman_array.len());

        let mut huffman_bytes: Vec<u8> = Vec::with_capacity(huffman_array.len() * 4);
        for i in 0..huffman_array.len() {
            let u = huffman_array[i as usize];
            let bytes: [u8; 4] = [
                ((u >> 0) & 0xFF) as u8,
                ((u >> 8) & 0xFF) as u8,
                ((u >> 16) & 0xFF) as u8,
                ((u >> 24) & 0xFF) as u8,
            ];
            for j in 0..4 {
                huffman_bytes.push(/*byte_reverse*/ bytes[j]);
            }
        }
        let tot_bits = huffman_array.len() * 32 - 32 + number_of_bits_used_in_last_integer as usize;

        let v = huffman_decode_i8(huffman_bytes, tot_bits, num_bits, true);
        assert_eq!(1950, v.len());
    }
}
