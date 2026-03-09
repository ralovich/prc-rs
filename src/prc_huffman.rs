// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

//use crate::prc_builtin;
use crate::prc_builtin::{UncompressedUnsignedInteger, UnsignedCharacter, UnsignedInteger};
//use crate::prc_gen::Entity_schema_definition;
use bitstream_io::{BitRead, BitReader, LittleEndian};
//use num_enum::TryFromPrimitive;
//use std::collections::HashMap;
//use std::convert::TryFrom;
use std::io::{Cursor, SeekFrom};
use std::{fmt, io};
use crate::function;


#[allow(unused)]
fn byte_reverse(mut b: u8) -> u8 {
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    b
}

pub fn read_huffman_to_element_array_i8<R: BitRead>(
    r: &mut R,
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
) -> io::Result<Vec<i8>> {
    //dbg!(has_is_compressed_bit, num_bits_per_elem, is_compressed_dv);
    let mut is_compressed = is_compressed_dv;
    if has_is_compressed_bit {
        is_compressed = r.read_bit()?;
    }
    //dbg!(is_compressed);

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
    let mut huffman_array: Vec<u32> = Vec::with_capacity(huffman_array_size as usize);
    for _i in 0..huffman_array_size {
        let ui = UncompressedUnsignedInteger::from_reader(r)?.value;
        huffman_array.push(ui);
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    assert!(number_of_bits_used_in_last_integer < 32);

    // let mut tot_bits = 0;
    // let mut huffman_bits: Vec<bool> = Vec::new();
    // for i in 0..huffman_array_size {
    //     let num_bits_in_ui = if i+1 == huffman_array_size { number_of_bits_used_in_last_integer } else { 32 };
    //     let input = huffman_array[i as usize];
    //     for j in 0..num_bits_in_ui {
    //         let b = (input >> j) & 0x01 != 0;
    //         huffman_bits.push(b);
    //     }
    //     tot_bits += num_bits_in_ui;
    // }
    // dbg!(huffman_array_size*32, tot_bits);
    // assert_eq!(tot_bits as usize, huffman_bits.len());
    // assert_eq!(huffman_array_size*32, tot_bits + 32-number_of_bits_used_in_last_integer);

    let mut huffman_bytes: Vec<u8> = Vec::with_capacity(huffman_array_size as usize * 4);
    for i in 0..huffman_array_size {
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
    //dbg!(tot_bits);

    huffman_array.clear(); // release memory

    let v = huffman_decode_i8(huffman_bytes, tot_bits, num_bits_per_elem);
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
        is_compressed = r.read_bit()?;
    }
    //dbg!(is_compressed);

    if !is_compressed {
        let arr_size = UnsignedInteger::from_reader(r)?.value;
        //dbg!(arr_size);
        let mut v = Vec::with_capacity(arr_size as usize);
        for _i in 0..arr_size {
            let lo = UnsignedCharacter::from_reader(r)?.value as u16;
            let hi = UnsignedCharacter::from_reader(r)?.value as u16;
            v.push((hi<<8 | lo) as i16);
        }
        assert_eq!(v.len(), arr_size as usize);
        return Ok(v);
    }

    let huffman_array_size = UnsignedInteger::from_reader(r)?.value;
    //dbg!(huffman_array_size);
    let mut huffman_array: Vec<u32> = Vec::with_capacity(huffman_array_size as usize);
    for _i in 0..huffman_array_size {
        let ui = UncompressedUnsignedInteger::from_reader(r)?.value;
        huffman_array.push(ui);
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    assert!(number_of_bits_used_in_last_integer < 32);

    // let mut tot_bits = 0;
    // let mut huffman_bits: Vec<bool> = Vec::new();
    // for i in 0..huffman_array_size {
    //     let num_bits_in_ui = if i+1 == huffman_array_size { number_of_bits_used_in_last_integer } else { 32 };
    //     let input = huffman_array[i as usize];
    //     for j in 0..num_bits_in_ui {
    //         let b = (input >> j) & 0x01 != 0;
    //         huffman_bits.push(b);
    //     }
    //     tot_bits += num_bits_in_ui;
    // }
    // dbg!(huffman_array_size*32, tot_bits);
    // assert_eq!(tot_bits as usize, huffman_bits.len());
    // assert_eq!(huffman_array_size*32, tot_bits + 32-number_of_bits_used_in_last_integer);

    let mut huffman_bytes: Vec<u8> = Vec::with_capacity(huffman_array_size as usize * 4);
    for i in 0..huffman_array_size {
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
    //dbg!(tot_bits);

    huffman_array.clear(); // release memory

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
) -> Vec<i8> {
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
        let value = r.read_var::<i8>(num_bits_per_elem as u32).unwrap();
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
                //println!("Trialling (for elem #{}) @ {} bits: {:b} {:b} ", _i, nb, trial, leaves[j].code_value);
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
            println!("Could not decode value for element {}", _i);
            return vec![];
        }

        data.push(leaves[selected_leaf.unwrap() as usize].value);
    }
    assert_eq!(data.len(), elem_array_size as usize);
    if _tot_bits as u64 != r.position_in_bits().unwrap() {
        println!(
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
                //println!("Trialling (for elem #{}) @ {} bits: {:b} {:b} ", _i, nb, trial, leaves[j].code_value);
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
            println!("Could not decode value for element {}", _i);
            return vec![];
        }

        data.push(leaves[selected_leaf.unwrap() as usize].value);
    }
    assert_eq!(data.len(), elem_array_size as usize);
    if _tot_bits as u64 != r.position_in_bits().unwrap() {
        println!(
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
}
