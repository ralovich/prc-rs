// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use crate::builtin::{UnsignedCharacter, UnsignedInteger, UnsignedShort, read_bits, write_bits};
use crate::function;
use crate::test_common::fill_partial_byte_at_end;
use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use log::debug;
use measure_time::debug_time;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Cursor;
//use std::marker::PhantomData;
use std::{fmt, io};

#[allow(unused)]
fn byte_reverse(mut b: u8) -> u8 {
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    b
}

pub fn write_huffman_from_element_array_i8<W: BitWrite + ?Sized>(
    w: &mut W,
    elements: &[i8],
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
    sign_extend: bool,
) -> io::Result<()> {
    let is_compressed = is_compressed_dv;
    if has_is_compressed_bit {
        write_bits(w, is_compressed as u8, 1)?;
    }

    if !is_compressed {
        UnsignedInteger {
            value: elements.len() as u32,
        }
        .to_writer(w)?;
        for it in elements.iter() {
            write_bits(w, *it as u8, 8)?;
        }
    } else {
        let (prc_huffman_bytes, _padding_bits) =
            prc_huffman_encode_i8(elements, num_bits_per_elem, sign_extend)?;
        assert_eq!(prc_huffman_bytes.len() % 4, 0);
        UnsignedInteger {
            value: prc_huffman_bytes.len() as u32 / 4,
        }
        .to_writer(w)?;
        for it in prc_huffman_bytes.iter() {
            write_bits(w, *it, 8)?;
        }
        UnsignedInteger {
            value: _padding_bits as u32,
        }
        .to_writer(w)?;
    }

    Ok(())
}

pub fn write_huffman_from_element_array_i16<W: BitWrite + ?Sized>(
    w: &mut W,
    elements: &[i16],
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
    sign_extend: bool,
) -> io::Result<()> {
    let is_compressed = is_compressed_dv;
    if has_is_compressed_bit {
        write_bits(w, is_compressed as u8, 1)?;
    }

    if !is_compressed {
        UnsignedInteger {
            value: elements.len() as u32,
        }
        .to_writer(w)?;
        for it in elements.iter() {
            UnsignedShort { value: *it as u16 }.to_writer(w)?;
        }
    } else {
        let (prc_huffman_bytes, _padding_bits) =
            prc_huffman_encode_i16(elements, num_bits_per_elem, sign_extend)?;
        assert_eq!(prc_huffman_bytes.len() % 4, 0);
        UnsignedInteger {
            value: prc_huffman_bytes.len() as u32 / 4,
        }
        .to_writer(w)?;
        for it in prc_huffman_bytes.iter() {
            write_bits(w, *it, 8)?;
        }
        UnsignedInteger {
            value: _padding_bits as u32,
        }
        .to_writer(w)?;
    }

    Ok(())
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
    //debug!("is_compressed: {}", is_compressed);

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
    let mut huffman_bytes: Vec<u8> = vec![0; huffman_array_size as usize * 4];
    for it in huffman_bytes.iter_mut() {
        *it = read_bits(r, 8)?;
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    //assert!(number_of_bits_used_in_last_integer > 0); // fails 3D-PDF-Sample-School.stream-48.prc
    if number_of_bits_used_in_last_integer > 32 {
        return Err(std::io::Error::other(
            "number_of_bits_used_in_last_integer <= 32 failed",
        ));
    }
    assert!(number_of_bits_used_in_last_integer <= 32);

    let tot_bits = huffman_bytes.len() * 8 - 32 + number_of_bits_used_in_last_integer as usize;
    //dbg!(tot_bits);

    // TODO: huffman decode could run async
    huffman_decode_i8(&huffman_bytes, tot_bits, num_bits_per_elem, sign_extend)
}

pub fn read_huffman_to_element_array_i16<R: BitRead>(
    r: &mut R,
    has_is_compressed_bit: bool,
    num_bits_per_elem: u8,
    is_compressed_dv: bool,
    sign_extend: bool,
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
            let val = UnsignedShort::from_reader(r)?.value as i16;
            v.push(val);
        }
        assert_eq!(v.len(), arr_size as usize);
        return Ok(v);
    }

    let huffman_array_size = UnsignedInteger::from_reader(r)?.value;
    //dbg!(huffman_array_size);
    if huffman_array_size == 0 {
        return Ok(vec![]);
    }
    let mut huffman_bytes: Vec<u8> = vec![0; huffman_array_size as usize * 4];
    for it in huffman_bytes.iter_mut() {
        *it = read_bits(r, 8)?;
    }

    let number_of_bits_used_in_last_integer = UnsignedInteger::from_reader(r)?.value;
    //dbg!(number_of_bits_used_in_last_integer);
    //assert!(number_of_bits_used_in_last_integer > 0); // fails 3D-PDF-Sample-School.stream-48.prc
    if number_of_bits_used_in_last_integer > 32 {
        return Err(std::io::Error::other(
            "number_of_bits_used_in_last_integer <= 32 failed",
        ));
    }
    assert!(number_of_bits_used_in_last_integer <= 32);

    let tot_bits = huffman_bytes.len() * 8 - 32 + number_of_bits_used_in_last_integer as usize;
    //dbg!(tot_bits);

    // TODO: huffman decode could run async
    huffman_decode_i16(&huffman_bytes, tot_bits, num_bits_per_elem, sign_extend)
}

#[allow(unused)]
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
        rv <<= 1;
        let bit = val >> k & 0x01;
        rv |= bit;
    }
    rv
}

/// Leaf of a Huffman tree
#[derive(Clone, PartialEq)]
struct HuffTreeLeaf<T> {
    pub symbol: T,
    pub code_length: u8,
    pub code_value: u32,
}
impl fmt::Debug for HuffTreeLeaf<i8> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:5} {:8} {:016b}",
            self.symbol, self.code_length, self.code_value
        )
    }
}
impl fmt::Debug for HuffTreeLeaf<i16> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:7} {:8} {:016b}",
            self.symbol, self.code_length, self.code_value
        )
    }
}

/// Node in a binary Huffman tree.
#[derive(Debug, Default, PartialEq)]
struct HTNode<T> {
    freq: u32,
    symb: T,
    left: Option<Box<HTNode<T>>>,
    right: Option<Box<HTNode<T>>>,

    code_length: Option<u8>,
    code_value: Option<u32>,
}

impl<T: Eq + std::hash::Hash + Clone + Copy + Default + std::cmp::PartialEq + std::fmt::Display>
    HTNode<T>
{
    pub fn new_leaf(freq: u32, symb: T) -> Self {
        Self {
            freq,
            symb,
            left: None,
            right: None,
            code_length: None,
            code_value: None,
        }
    }
    pub fn new_internal(freq: u32, symb: T, left: Box<HTNode<T>>, right: Box<HTNode<T>>) -> Self {
        Self {
            freq,
            symb,
            left: Some(left),
            right: Some(right),
            code_length: None,
            code_value: None,
        }
    }
    /// build a tree from an input stream of symbols
    pub fn build(bytes: &[T]) -> Box<HTNode<T>> {
        // <symbol, frequency>
        let mut freq: HashMap<T, u32> = HashMap::new();
        for b in bytes {
            let _ = match freq.get(&b) {
                Some(count) => freq.insert(*b, count + 1),
                None => freq.insert(*b, 1),
            };
        }
        let mut freqs: Vec<(T, u32)> = Vec::new();
        for sf in freq.iter() {
            freqs.push((*sf.0, *sf.1));
        }
        freq.clear();
        freqs.sort_by(|a, b| b.1.cmp(&a.1));

        let mut nodes: Vec<Box<HTNode<T>>> = Vec::new();
        for (symb, freq) in &freqs {
            nodes.push(Box::new(HTNode::<T>::new_leaf(*freq, *symb)));
        }
        freqs.clear();
        while nodes.len() > 1 {
            // sort and merge
            nodes.sort_by(|a, b| b.freq.cmp(&a.freq));
            let left = nodes.pop().unwrap();
            let right = nodes.pop().unwrap();
            let merged = Box::new(HTNode::<T>::new_internal(
                left.freq + right.freq,
                Default::default(),
                left,
                right,
            ));
            nodes.push(merged);
        }
        assert_eq!(nodes.len(), 1usize);
        nodes.pop().unwrap()
    }
    /// Collect leaves of a fully built tree.
    pub fn collect_leaves(&self, leaves: &mut Vec<HuffTreeLeaf<T>>, edge_code: u32, edge_len: u8) {
        if self.left.is_none() && self.right.is_none() {
            leaves.push(HuffTreeLeaf::<T> {
                symbol: self.symb,
                code_length: edge_len,
                code_value: edge_code,
            });
        } else {
            if let Some(left) = &self.left {
                left.collect_leaves(leaves, edge_code, edge_len + 1);
            }
            if let Some(right) = &self.right {
                right.collect_leaves(leaves, edge_code | (1 << edge_len), edge_len + 1);
            }
        }
    }
    pub fn insert(
        root: &mut Box<HTNode<T>>,
        code_length: u8,
        code_value: u32,
        symbol: T,
    ) -> Result<(), ()> {
        let mut node = root;
        for bit_index in 1..=code_length {
            let mask0 = 1 << (bit_index - 1);
            //let mask = !(mask0);
            let bit = (code_value & mask0) != 0;
            let last = bit_index == code_length;
            //println!(
            //    "{:016b} {:016b} bit={} last={}",
            //    code_value, mask0, bit, last
            //);
            if last {
                if bit {
                    if node.right.is_some() {
                        return Err(());
                    }
                    assert!(node.right.is_none());
                    node.right = Some(Box::new(HTNode::<T>::new_leaf(0, symbol)));
                    node.right.as_mut().unwrap().code_length = Some(code_length);
                    node.right.as_mut().unwrap().code_value = Some(code_value);
                } else {
                    if node.left.is_some() {
                        return Err(());
                    }
                    assert!(node.left.is_none());
                    node.left = Some(Box::new(HTNode::<T>::new_leaf(0, symbol)));
                    node.left.as_mut().unwrap().code_length = Some(code_length);
                    node.left.as_mut().unwrap().code_value = Some(code_value);
                }
            } else {
                if bit {
                    if node.right.is_none() {
                        node.right = Some(Box::default());
                    }
                    node = node.right.as_mut().unwrap();
                    //Self::insert(node.right, code_length, code_value);
                } else {
                    if node.left.is_none() {
                        node.left = Some(Box::default());
                    }
                    node = node.left.as_mut().unwrap();
                }
            }
            // if bit_index == 0 {
            //     break;
            // }
            //bit_index -= 1;
        }
        //assert_eq!(bit_index, code_length);
        // let bit = code_value & !(1 << (bit_index - 1));
        // if bit != 0 {
        //     assert!(node.right.is_none());
        //     node.right = Some(Box::new(HTNode::<T>::new_leaf(0, symbol)));
        // } else {
        //     assert!(node.left.is_none());
        //     node.left = Some(Box::new(HTNode::<T>::new_leaf(0, symbol)));
        // }
        Ok(())
    }
    /// Re-assemble a tree from the collection of leaves.
    pub fn tree_from_leaves(leaves: &[HuffTreeLeaf<T>]) -> Option<Box<HTNode<T>>>
    where
        HuffTreeLeaf<T>: std::fmt::Debug,
    {
        if leaves.is_empty() {
            return None;
        }
        let mut root = Box::new(HTNode::<T>::new_leaf(0, Default::default()));
        for leaf in leaves.iter() {
            //Self::insert_into_tree(&mut root, &leaf);
            match Self::insert(&mut root, leaf.code_length, leaf.code_value, leaf.symbol) {
                Ok(_) => {}
                Err(()) => return None,
            }
        }
        Some(root)
    }
    /// Try to read a Huffman code bit-by-bit and return the corresponding decoded symbol.
    pub fn code_from_reader_as_symbol<R: BitRead>(
        r: &mut R,
        node: &Box<HTNode<T>>,
        edge_code: u32,
        edge_len: u8,
    ) -> Option<T> {
        if node.left.is_none() && node.right.is_none() {
            return Some(node.symb.clone());
        }
        let bit = r.read_bit();
        if bit.is_err() {
            return None;
        }
        let bit = bit.unwrap();
        if bit {
            //println!("1");
            // go right
            if node.right.is_some() {
                return Self::code_from_reader_as_symbol(
                    r,
                    &node.right.as_ref().unwrap(),
                    edge_code | (1 << edge_len),
                    edge_len + 1,
                );
            } else {
                // this is a leaf
                if node.code_length.is_none() || node.code_value.is_none() {
                    return None;
                }
                assert_eq!(edge_len, node.code_length.unwrap());
                // if edge_code != node.code_value.unwrap() {
                //     println!("{:016b} {:016b}", edge_code, node.code_value.unwrap());
                // }
                assert_eq!(edge_code, node.code_value.unwrap());
                // println!(
                //     "Found2 {} {} {:016b}",
                //     node.symb,
                //     node.code_length.unwrap(),
                //     node.code_value.unwrap()
                // );
                return Some(node.symb.clone());
            }
        } else {
            //println!("0");
            // go left
            if node.left.is_some() {
                return Self::code_from_reader_as_symbol(
                    r,
                    &node.left.as_ref().unwrap(),
                    edge_code,
                    edge_len + 1,
                );
            } else {
                if node.code_length.is_none() || node.code_value.is_none() {
                    return None;
                }
                assert_eq!(edge_len, node.code_length.unwrap());
                // if edge_code != node.code_value.unwrap() {
                //     println!("{:016b} {:016b}", edge_code, node.code_value.unwrap());
                // }
                assert_eq!(edge_code, node.code_value.unwrap());
                // println!(
                //     "Found2 {} {} {:016b}",
                //     node.symb,
                //     node.code_length.unwrap(),
                //     node.code_value.unwrap()
                // );
                return Some(node.symb.clone());
            }
        }
    }
}

pub fn prc_huffman_encode<
    T: PartialEq
        + std::cmp::Eq
        + std::hash::Hash
        + std::marker::Copy
        + std::default::Default
        + Ord
        + std::fmt::Display
        + std::ops::BitAnd<Output = T>
        + Sized,
    WriteT: PartialEq + bitstream_io::Integer + std::ops::BitAnd<Output = WriteT> + Sized,
>(
    symbols: &[T],
    num_bits_per_elem: u8,
    _sign_extend: bool,
    mask: T,
) -> std::io::Result<(Vec<u8>, usize)> {
    debug_time!("prc_huffman_encode<T>");
    //assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 8);
    //dbg!(num_bits_per_elem);

    let root = HTNode::build(symbols);
    let mut leaves: Vec<HuffTreeLeaf<T>> = Vec::new();
    root.collect_leaves(&mut leaves, 1, 1);
    leaves.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    // for leaf in leaves.iter_mut() {
    //     leaf.code_value = rev_bits(leaf.code_value, leaf.code_length);
    // }
    // dbg!(&leaves);
    // for leaf in leaves.iter_mut() {
    //     leaf.code_value = rev_bits(leaf.code_value, leaf.code_length);
    // }

    let mut prc_huffman_bytes: Vec<u8> = vec![];
    let mut w = BitWriter::endian(&mut prc_huffman_bytes, bitstream_io::LittleEndian);
    let num_leaves = leaves.len() as u32;
    //dbg!(num_leaves);
    w.write_var::<u32>(num_bits_per_elem as u32 + 1, num_leaves)?;
    //let max_code_length = leaves.iter().map(|x| x.code_length).max().unwrap();
    let max_code_length = crate::builtin::get_number_of_bits_used_to_store_unsigned_integer(
        leaves.iter().map(|x| x.code_length).max().unwrap() as u32,
    ) as u8
        + 1;
    //dbg!(max_code_length);
    w.write_var::<u8>(8, max_code_length)?;
    for i in 0..num_leaves as usize {
        let symb = leaves[i].symbol;
        let code_length = leaves[i].code_length;
        let code_value = rev_bits(leaves[i].code_value, code_length);
        //let mask = ((1u32 << num_bits_per_elem) - 1) as u8;
        //assert_ne!(mask, 0);
        //if _sign_extend {
        //} else {
        //}
        //let val: WriteT = MyInto::<T, WriteT>::my_as(symb & mask);
        let masked = symb & mask;
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<WriteT>());
        let val: WriteT = unsafe { std::mem::transmute_copy::<T, WriteT>(&masked) };
        w.write_var::<WriteT>(num_bits_per_elem as u32, val)?;
        w.write_var::<u32>(max_code_length as u32, code_length as u32)?;
        w.write_var::<u32>(code_length as u32, code_value as u32)?;
    }
    w.write_var::<u32>(32, symbols.len() as u32)?;
    for symb in symbols {
        let idx = leaves
            .iter()
            .position(|x| x.symbol == *symb)
            .ok_or(std::io::Error::other("no leaf for symbol"))?;
        let code = leaves[idx].code_value;
        let code_num_bits = leaves[idx].code_length as u32;
        w.write_unsigned_var::<u32>(code_num_bits, code)?;
    }

    let mut _padding_bits = fill_partial_byte_at_end(&mut w, false)?;
    while prc_huffman_bytes.len() % 4 != 0 {
        prc_huffman_bytes.push(0);
        _padding_bits += 8;
    }

    Ok((prc_huffman_bytes, _padding_bits))
}

pub fn prc_huffman_encode_i8(
    symbols: &[i8],
    num_bits_per_elem: u8,
    _sign_extend: bool,
) -> std::io::Result<(Vec<u8>, usize)> {
    debug_time!("prc_huffman_encode_i8");
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 8);
    //dbg!(num_bits_per_elem);

    prc_huffman_encode::<i8, u8>(
        symbols,
        num_bits_per_elem,
        _sign_extend,
        get_mask_i8(num_bits_per_elem),
    )
}

pub fn prc_huffman_encode_i16(
    symbols: &[i16],
    num_bits_per_elem: u8,
    _sign_extend: bool,
) -> std::io::Result<(Vec<u8>, usize)> {
    debug_time!("prc_huffman_encode_i16");
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 16);
    //dbg!(num_bits_per_elem);

    prc_huffman_encode::<i16, u16>(
        symbols,
        num_bits_per_elem,
        _sign_extend,
        get_mask_i16(num_bits_per_elem),
    )
}

fn get_mask_i8(num_bits_per_elem: u8) -> i8 {
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 8);
    let mask = (1u32 << num_bits_per_elem) - 1;
    assert_ne!(mask, 0);
    let mask_t = mask as i8;
    mask_t
}
fn get_mask_i16(num_bits_per_elem: u8) -> i16 {
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 16);
    let mask = (1u32 << num_bits_per_elem) - 1;
    assert_ne!(mask, 0);
    let mask_t = mask as i16;
    mask_t
}

fn huffman_decode<
    T: PartialEq
        + Ord
        + bitstream_io::Integer
        + std::hash::Hash
        + Copy
        + Default
        + Display
        + std::fmt::Debug
        + std::ops::BitAnd<Output = T>,
>(
    prc_huffman_bytes: &Vec<u8>,
    _tot_bits: usize,
    num_bits_per_elem: u8,
    sign_extend: bool,
    mask: T,
) -> io::Result<Vec<T>>
where
    crate::huffman::HuffTreeLeaf<T>: std::fmt::Debug,
{
    //debug_time!("huffman_decode<T>");
    //assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 8);
    //dbg!(num_bits_per_elem);

    let mut r = BitReader::endian(Cursor::new(&prc_huffman_bytes), LittleEndian);
    let num_leaves = r.read_var::<u32>(num_bits_per_elem as u32 + 1)?;
    //dbg!(num_leaves);
    if num_leaves == 0 {
        return Ok(vec![]);
    }

    let max_code_length = r.read_var::<u8>(8)?;
    //dbg!(max_code_length);
    #[allow(clippy::absurd_extreme_comparisons)]
    if max_code_length <= 0 {
        return Err(std::io::Error::other("max_code_length <= 0 failed!"));
    }
    if max_code_length > 32 {
        return Err(std::io::Error::other("max_code_length > 32 failed!"));
    }
    assert!(max_code_length > 0 && max_code_length <= 32);

    let mut leaves: Vec<HuffTreeLeaf<T>> = Vec::with_capacity(num_leaves as usize);
    for _i in 0..num_leaves {
        let symbol: T;
        if sign_extend {
            symbol = r.read_var::<T>(num_bits_per_elem as u32)?;
        } else {
            //let mask = (1u32 << num_bits_per_elem) - 1;
            //assert_ne!(mask, 0);
            //let mask = T::try_from(mask).unwrap();
            let val = r.read_var::<T>(num_bits_per_elem as u32)?;
            symbol = val & mask;
        }
        let code_length = r.read_var::<u32>(max_code_length as u32)? as u8;
        let code_value = r.read_var::<u32>(code_length as u32)?;
        leaves.push(HuffTreeLeaf {
            symbol,
            code_length,
            code_value,
        });
    }
    //dbg!(&leaves.len(), min_code_bits, max_code_bits);
    //dbg!(&leaves);
    leaves.sort_by(|a, b| a.code_value.cmp(&b.code_value));
    //dbg!(&leaves);

    for leaf in leaves.iter_mut() {
        leaf.code_value = rev_bits(leaf.code_value, leaf.code_length);
    }
    let root = HTNode::tree_from_leaves(&leaves);
    if root.is_none() {
        return Err(std::io::Error::other(
            "Could not build Huffman tree from leaves!",
        ));
    }
    let root = root.unwrap();

    let elem_array_size = r.read::<32, u32>()?;
    //dbg!(elem_array_size);
    let mut data: Vec<T> = Vec::with_capacity(elem_array_size as usize);
    for _i in 0..elem_array_size {
        let symbol = HTNode::code_from_reader_as_symbol(&mut r, &root, 0, 0);
        if symbol.is_none() {
            return Err(std::io::Error::other("Could not decode!"));
        }
        let symbol = symbol.unwrap();
        data.push(symbol);
    }
    assert_eq!(data.len(), elem_array_size as usize);
    if _tot_bits as u64 != r.position_in_bits()? {
        debug!(
            "{}: {} bits consumed of {} bits",
            function!(),
            r.position_in_bits()?,
            _tot_bits
        );
    }

    Ok(data)
}

pub fn huffman_decode_i8(
    prc_huffman_bytes: &Vec<u8>,
    _tot_bits: usize,
    num_bits_per_elem: u8,
    sign_extend: bool,
) -> io::Result<Vec<i8>> {
    //debug_time!("huffman_decode_i8");
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 8);
    //dbg!(num_bits_per_elem);

    huffman_decode::<i8>(
        prc_huffman_bytes,
        _tot_bits,
        num_bits_per_elem,
        sign_extend,
        get_mask_i8(num_bits_per_elem),
    )
}
pub fn huffman_decode_i16(
    prc_huffman_bytes: &Vec<u8>,
    _tot_bits: usize,
    num_bits_per_elem: u8,
    sign_extend: bool,
) -> io::Result<Vec<i16>> {
    //debug_time!("huffman_decode_i16");
    assert!(num_bits_per_elem > 0 && num_bits_per_elem <= 16);

    huffman_decode::<i16>(
        prc_huffman_bytes,
        _tot_bits,
        num_bits_per_elem,
        sign_extend,
        get_mask_i16(num_bits_per_elem),
    )
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
    fn test_huffman_tree() {
        let s = "3D-PDF-Sample-Aero-Composite-Part.stream-48 point_array in File Structure 0: section TESSELLATION_SECTION: 65438 bytes".to_owned();
        let bytes = s.as_bytes().iter().map(|b| *b as i8).collect::<Vec<_>>();
        assert_eq!(bytes.len(), 118);

        // build tree from symbols
        let root = HTNode::build(bytes.as_slice());
        let mut leaves: Vec<HuffTreeLeaf<i8>> = Vec::new();
        root.collect_leaves(&mut leaves, 1, 1);
        leaves.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        // rebuild tree from leaves
        let root2 = HTNode::tree_from_leaves(&leaves).unwrap();
        let mut leaves2: Vec<HuffTreeLeaf<i8>> = Vec::new();
        root2.collect_leaves(&mut leaves2, 0, 0);
        leaves2.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        assert_eq!(leaves2, leaves);
    }

    #[test]
    fn test_huffman_decode() {
        let num_bits = 6u8;
        // 3D-PDF-Sample-Aero-Composite-Part.stream-48 point_array in File Structure 0: section TESSELLATION_SECTION: 65438 bytes
        let number_of_bits_used_in_last_integer = 17usize;
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
            let u = huffman_array[i];
            let bytes: [u8; 4] = [
                ((u >> 0) & 0xFF) as u8,
                ((u >> 8) & 0xFF) as u8,
                ((u >> 16) & 0xFF) as u8,
                ((u >> 24) & 0xFF) as u8,
            ];
            for j in 0..4 {
                huffman_bytes.push(bytes[j]);
            }
        }
        assert_eq!(1152, huffman_bytes.len());
        let tot_bits = huffman_array.len() * 32 - 32 + number_of_bits_used_in_last_integer as usize;

        let v = huffman_decode_i8(&huffman_bytes, tot_bits, num_bits, true).unwrap();
        assert_eq!(1950, v.len());

        {
            // round-trip, make sure encoder produces the same bitstream as found in an existing PRC
            let (encoded, num_padding_bits) =
                prc_huffman_encode_i8(v.as_slice(), num_bits, true).unwrap();
            assert_eq!(encoded.len(), huffman_bytes.len());
            assert_eq!(&encoded, &huffman_bytes);
            assert_eq!(32 - num_padding_bits, number_of_bits_used_in_last_integer);
        }
    }

    #[test]
    fn test_huffman_encode() {
        let s = "3D-PDF-Sample-Aero-Composite-Part.stream-48 point_array in File Structure 0: section TESSELLATION_SECTION: 65438 bytes".to_owned();
        let bytes = s.as_bytes().iter().map(|b| *b as i8).collect::<Vec<_>>();
        assert_eq!(bytes.len(), 118);

        let (prc_huffman_bytes, _padding_bits) =
            prc_huffman_encode_i8(bytes.as_slice(), 8, false).unwrap();

        {
            // round trip, make sure encoder and decoder talk the same "language"
            let _tot_bits = prc_huffman_bytes.len() * 8 - _padding_bits;
            let recovered_i8 = huffman_decode_i8(&prc_huffman_bytes, _tot_bits, 8, false).unwrap();
            let recovered = recovered_i8.iter().map(|x| *x as u8).collect::<Vec<_>>();
            assert_eq!(recovered.len(), s.len());
            let s_recovered = String::from_utf8(recovered).unwrap();
            assert_eq!(s_recovered, s);
        }
    }

    #[test]
    fn test_huffman_i16() {
        let symbols = [1i16, 2i16, 3i16, 4i16, 5i16, -1116i16, 2i16, 2i16, -349i16];
        let num_bits_per_elem = 12;
        let sign_extend = true;
        let (prc_huffman_bytes, _padding_bits) =
            prc_huffman_encode_i16(&symbols, num_bits_per_elem, sign_extend).unwrap();

        {
            // round trip, make sure encoder and decoder talk the same "language"
            let _tot_bits = prc_huffman_bytes.len() * 8 - _padding_bits;
            let recovered = huffman_decode_i16(
                &prc_huffman_bytes,
                _tot_bits,
                num_bits_per_elem,
                sign_extend,
            )
            .unwrap();
            //let recovered = recovered_i16.iter().map(|x| *x as u8).collect::<Vec<_>>();
            assert_eq!(recovered.len(), symbols.len());
            assert_eq!(recovered, symbols);
        }
    }
}
