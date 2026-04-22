// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use crate::constants::PrcType;
use crate::prc_builtin;
use crate::prc_gen::Entity_schema_definition;
use bitstream_io::BitReader;
use log::debug;
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
//use crate::prc_schema::VariableKind::Double;

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Debug, TryFromPrimitive)]
enum SchemaTokens {
    EPRCSchema_Data_Boolean = 0,
    EPRCSchema_Data_Double = 1,
    EPRCSchema_Data_Character = 2,
    EPRCSchema_Data_Unsigned_Integer = 3,
    EPRCSchema_Data_Integer = 4,
    EPRCSchema_Data_String = 5,
    EPRCSchema_Father_Type = 6,
    EPRCSchema_Vector_2D = 7,
    EPRCSchema_Vector_3D = 8,
    EPRCSchema_Extent_1D = 9,
    EPRCSchema_Extent_2D = 10,
    EPRCSchema_Extent_3D = 11,
    EPRCSchema_Ptr_Type = 12,
    EPRCSchema_Ptr_Surface = 13,
    EPRCSchema_Ptr_Curve = 14,
    EPRCSchema_For = 15,
    EPRCSchema_SimpleFor = 16,
    EPRCSchema_If = 17,
    EPRCSchema_Else = 18,
    EPRCSchema_Block_Start = 19,
    EPRCSchema_Block_Version = 20,
    EPRCSchema_Block_End = 21,
    EPRCSchema_Value_Declare = 22,
    EPRCSchema_Value_Set = 23,
    EPRCSchema_Value_DeclareAndSet = 24,
    EPRCSchema_Value = 25,
    EPRCSchema_Value_Constant = 26,
    EPRCSchema_Value_For = 27,
    EPRCSchema_Value_CurveIs3D = 28,
    EPRCSchema_Operator_MULT = 29,
    EPRCSchema_Operator_DIV = 30,
    EPRCSchema_Operator_ADD = 31,
    EPRCSchema_Operator_SUB = 32,
    EPRCSchema_Operator_LT = 33,
    EPRCSchema_Operator_LE = 34,
    EPRCSchema_Operator_GT = 35,
    EPRCSchema_Operator_GE = 36,
    EPRCSchema_Operator_EQ = 37,
    EPRCSchema_Operator_NEQ = 38,
    //Obsolete schema token that may exist in older files. The data field is of type index and should be ignored.
    EPRCSchema_ObsoleteToken39NextToIgnore = 39, // 39 is legacy, see https://github.com/pdf-association/pdf-issues/issues/407 : Obsolete schema token that may exist in older files. The data field is of type index and should be ignored.
    EPRCSchema_ObsoleteToken40NextToIgnore = 40, // 40 is legacy, see https://github.com/pdf-association/pdf-issues/issues/407 : Obsolete schema token that may exist in older files. The data field is of type index and should be ignored.
}
impl fmt::Display for SchemaTokens {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}
pub fn print_schema(tokens: &Vec<u32>) {
    use SchemaTokens::*;

    let mut indent: u32 = 4;
    let mut prev_raw: bool = false;
    for j in 0..tokens.len() {
        let tok = tokens[j];
        if tok == EPRCSchema_Block_End as u32 || tok == EPRCSchema_Else as u32 {
            if indent > 0 {
                indent -= 2;
            }
        }
        let mut show_raw: bool = false;
        if j > 0 {
            let prev_tok = tokens[j - 1];
            if !prev_raw
                && (prev_tok == EPRCSchema_Block_Version as u32
                    || prev_tok == EPRCSchema_Value_Constant as u32
                    || prev_tok == EPRCSchema_Value_Declare as u32
                    || prev_tok == EPRCSchema_Value_Set as u32
                    || prev_tok == EPRCSchema_Value_DeclareAndSet as u32
                    || prev_tok == EPRCSchema_Value as u32
                    || prev_tok == EPRCSchema_ObsoleteToken39NextToIgnore as u32
                    || prev_tok == EPRCSchema_ObsoleteToken40NextToIgnore as u32
                    || prev_tok == EPRCSchema_Father_Type as u32)
            {
                show_raw = true;
            }
        }
        if show_raw {
            debug!(
                "{}{}",
                (0..indent).map(|_| " ").collect::<String>(),
                tokens[j]
            );
        } else {
            debug!(
                "{}{}",
                (0..indent).map(|_| " ").collect::<String>(),
                SchemaTokens::try_from(tok).unwrap().to_string()
            );
        }
        if tok == EPRCSchema_Block_Start as u32
            || tok == EPRCSchema_Block_Version as u32
            || tok == EPRCSchema_If as u32
            || tok == EPRCSchema_Else as u32
            || tok == EPRCSchema_For as u32
            || tok == EPRCSchema_SimpleFor as u32
        {
            indent += 2;
        }
        prev_raw = show_raw;
    }
    debug!("");
    ()
}

#[derive(Debug, Clone)]
pub enum VariableKind {
    Invalid,
    Boolean(bool),
    Double(f64),
    Char(u8),
    Unsigned(u32),
    Integer(i32),
    String(String),
    Vector2D(f64, f64),
    Vector3D(f64, f64, f64),
}
impl VariableKind {
    fn is_scalar(&self) -> bool {
        match self {
            VariableKind::Boolean(_x) => true,
            VariableKind::Double(_x) => true,
            VariableKind::Char(_x) => true,
            VariableKind::Unsigned(_x) => true,
            VariableKind::Integer(_x) => true,
            _ => false,
        }
    }
    fn as_scalar(&self) -> f64 {
        match self {
            VariableKind::Boolean(x) => {
                if *x {
                    1.0
                } else {
                    0.0
                }
            }
            VariableKind::Double(x) => *x,
            VariableKind::Char(x) => *x as f64,
            VariableKind::Unsigned(x) => *x as f64,
            VariableKind::Integer(x) => *x as f64,
            _ => panic!("not a scalar!"),
        }
    }
}
impl Default for VariableKind {
    fn default() -> VariableKind {
        VariableKind::Invalid
    }
}
impl PartialEq for VariableKind {
    fn eq(&self, other: &Self) -> bool {
        let is_scalar = self.is_scalar();
        let other_scalar = other.is_scalar();
        if is_scalar && other_scalar {
            return self.as_scalar() == other.as_scalar();
        }
        match (self, other) {
            (VariableKind::String(x), VariableKind::String(y)) => x == y,
            (VariableKind::Vector2D(x1, x2), VariableKind::Vector2D(y1, y2)) => {
                x1 == y1 && x2 == y2
            }
            (VariableKind::Vector3D(x1, x2, x3), VariableKind::Vector3D(y1, y2, y3)) => {
                x1 == y1 && x2 == y2 && x3 == y3
            }
            _ => panic!("not implemented to handle non-scalar equality!"),
        }
    }
}
impl Eq for VariableKind {}

#[derive(Default, Debug)]
struct VmState {
    pub indent: u32,
    pub opstack: Vec<u32>,                // stack of instructions to execute
    pub dstack: Vec<VariableKind>,        // stack of data
    pub vars: HashMap<u32, VariableKind>, // map of <variable id, data> pairs
}
impl VmState {
    pub fn i(&self) -> std::string::String {
        (0..self.indent).map(|_| " ").collect::<String>()
    }
    pub fn merge_from(&mut self, other: &VmState) {
        let len_before = self.dstack.len();
        for v in other.dstack.iter().rev() {
            self.dstack.push(v.clone());
        }
        assert_eq!(self.dstack.len(), len_before + other.dstack.len());
        if !other.dstack.is_empty() {
            assert_eq!(self.dstack.last().unwrap(), other.dstack.first().unwrap());
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct SchemaEvaluator {
    pub ops_per_type: HashMap<u32, Vec<u32>>, // key cannot be PRCType as there might be new ids, like 801
    //s: VmState,
    stored_version: u32,
}
impl SchemaEvaluator {
    pub fn new(inp: &Vec<Entity_schema_definition>) -> SchemaEvaluator {
        let mut ops_per_type: HashMap<u32, Vec<u32>> = HashMap::new();
        for sch in inp {
            let _type_name = match PrcType::try_from(sch.entity_type.value) {
                Err(_) => sch.entity_type.value.to_string(),
                _ => PrcType::try_from(sch.entity_type.value)
                    .unwrap()
                    .to_string(),
            };
            debug!("Schema present for {}", _type_name);
            let mut v = Vec::new();
            for token in &sch.schema_tokens {
                v.push(token.value);
            }
            debug!("    {} tokens", v.len());
            print_schema(&v);
            ops_per_type.insert(sch.entity_type.value, v);
        }

        SchemaEvaluator {
            ops_per_type,
            //s: Default::default(),
            stored_version: 8137,
        }
    }

    fn merge_from(&mut self, _other: &SchemaEvaluator) {
        // TODO
    }
    pub fn eval<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        &mut self,
        rdr: &mut BitReader<R, E>,
        type_to_eval: u32,
        skip: bool,
        indent: u32,
    ) -> Result<Vec<VariableKind>, E> {
        // no instructions for this type
        if !self.ops_per_type.contains_key(&type_to_eval) {
            //println!("No schema to evaluate for {}", type_to_eval);
            return Ok(Vec::new());
        }

        // instructions for this type
        let ops = &self.ops_per_type[&type_to_eval];
        let type_name = match PrcType::try_from(type_to_eval) {
            Ok(t) => t.to_string(),
            Err(_) => type_to_eval.to_string(),
        };
        let mut s: VmState = Default::default();
        s.indent = indent;
        debug!(
            "{}Evaluating schema for {} ({} tokens)",
            s.i(),
            type_name,
            ops.len()
        );
        //print_schema(&ops);

        //self.s = Default::default();
        for &op in ops.iter().rev() {
            /*self.*/
            s.opstack.push(op);
        }
        assert_eq!(
            /*self.*/ s.opstack.first().unwrap(),
            ops.last().unwrap()
        );

        //while let Some(op) = self.s.opstack.pop() {
        while !/*self.*/s.opstack.is_empty() {
            self.do_eval(rdr, &mut /*self.*/s, skip);
        }
        assert!(s.opstack.is_empty());

        let mut variables_to_return = Vec::new();
        for v in s.dstack.iter() {
            variables_to_return.push(v.clone());
        }

        Ok(variables_to_return)
    }

    fn do_eval<R: std::io::Read + std::io::Seek, E: bitstream_io::Endianness>(
        &mut self,
        rdr: &mut BitReader<R, E>,
        s: &mut VmState,
        skip: bool,
    ) {
        use SchemaTokens::*;

        if s.opstack.is_empty() {
            return ();
        }

        s.indent += 1;
        let instr = s.opstack.pop().unwrap();
        match instr {
            val if val == EPRCSchema_Data_Boolean as u32 => {
                if skip {
                    debug!("{}SKIP READB", s.i());
                } else {
                    let tmp = prc_builtin::Boolean::from_reader(rdr).unwrap().value;
                    debug!("{}READB {}", s.i(), tmp);
                    s.dstack.push(VariableKind::Boolean(tmp));
                }
            }
            val if val == EPRCSchema_Data_Double as u32 => {
                if skip {
                    debug!("{}SKIP READD", s.i());
                } else {
                    let tmp = prc_builtin::Double::from_reader(rdr).unwrap().value;
                    debug!("{}READD {}", s.i(), tmp);
                    s.dstack.push(VariableKind::Double(tmp));
                }
            }
            val if val == EPRCSchema_Data_Character as u32 => {
                if skip {
                    debug!("{}SKIP READC", s.i());
                } else {
                    let tmp = prc_builtin::UnsignedCharacter::from_reader(rdr)
                        .unwrap()
                        .value;
                    debug!("{}READC {}", s.i(), tmp);
                    s.dstack.push(VariableKind::Char(tmp));
                }
            }
            val if val == EPRCSchema_Data_Unsigned_Integer as u32 => {
                if skip {
                    debug!("{}SKIP READU", s.i());
                } else {
                    let tmp = prc_builtin::UnsignedInteger::from_reader(rdr)
                        .unwrap()
                        .value;
                    debug!("{}READU {}", s.i(), tmp);
                    s.dstack.push(VariableKind::Unsigned(tmp));
                }
            }
            val if val == EPRCSchema_Data_Integer as u32 => {
                if skip {
                    debug!("{}SKIP READI", s.i());
                } else {
                    let tmp = prc_builtin::Integer::from_reader(rdr).unwrap().value;
                    debug!("{}READI {}", s.i(), tmp);
                    s.dstack.push(VariableKind::Integer(tmp));
                }
            }
            val if val == EPRCSchema_Data_String as u32 => {
                if skip {
                    debug!("{}SKIP READS", s.i());
                } else {
                    let tmp = prc_builtin::String::from_reader(rdr).unwrap().value;
                    debug!("{}READS \"{}\"", s.i(), tmp);
                    s.dstack.push(VariableKind::String(tmp));
                }
            }
            val if val == EPRCSchema_Father_Type as u32 => {
                let father_type_id = s.opstack.pop().unwrap();
                if skip {
                    debug!("{}SKIP FATHER {}", s.i(), father_type_id);
                } else {
                    debug!("{}FATHER {}", s.i(), father_type_id);
                    let mut nested = self.clone();
                    let _ = nested.eval(rdr, father_type_id, skip, s.indent);
                    self.merge_from(&mut nested);
                    //panic!("EPRCSchema_Father_Type not yet implemented!");
                }
            }
            val if val == EPRCSchema_Vector_2D as u32 => {
                if skip {
                    debug!("{}SKIP READV2D", s.i());
                } else {
                    let tmp: [f64; 2] = [
                        prc_builtin::Double::from_reader(rdr).unwrap().value,
                        prc_builtin::Double::from_reader(rdr).unwrap().value,
                    ];
                    debug!("{}READV2D {:?}", s.i(), tmp);
                    s.dstack.push(VariableKind::Vector2D(tmp[0], tmp[1]));
                }
            }
            val if val == EPRCSchema_Vector_3D as u32 => {
                if skip {
                    debug!("{}SKIP READV3D", s.i());
                } else {
                    let tmp: [f64; 3] = [
                        prc_builtin::Double::from_reader(rdr).unwrap().value,
                        prc_builtin::Double::from_reader(rdr).unwrap().value,
                        prc_builtin::Double::from_reader(rdr).unwrap().value,
                    ];
                    debug!("{}READV3D {:?}", s.i(), tmp);
                    s.dstack
                        .push(VariableKind::Vector3D(tmp[0], tmp[1], tmp[2]));
                }
            }
            val if val == EPRCSchema_For as u32 => {
                if skip {
                    debug!("{}SKIP FOR", s.i());
                    self.do_eval(rdr, s, true); // skip number of iterations
                    self.do_eval(rdr, s, true); // skip loop body
                } else {
                    self.do_eval(rdr, s, skip); // retrieve number of iterations
                    let n = match s.dstack.pop().unwrap() {
                        VariableKind::Integer(i) => i,
                        VariableKind::Unsigned(u) => u as i32,
                        _ => panic!("EPRCSchema_For unsupported iteration limit type!"),
                    };
                    debug!("{}FOR {:?}", s.i(), n);
                    // eval next instruction or block N times in a loop
                    for _i in 0..n - 1 {
                        //panic!("EPRCSchema_For not yet implemented!");
                        let mut s_tmp = VmState {
                            indent: s.indent,
                            opstack: s.opstack.clone(),
                            dstack: Default::default(),
                            vars: Default::default(),
                        };
                        self.do_eval(rdr, &mut s_tmp, false);
                        s.merge_from(&mut s_tmp);
                    }
                    if n > 0 {
                        self.do_eval(rdr, s, false);
                    } else {
                        self.do_eval(rdr, s, true);
                    }
                }
            }
            val if val == EPRCSchema_SimpleFor as u32 => {
                if skip {
                    debug!("{}SKIP SIMPLEFOR", s.i());
                    self.do_eval(rdr, s, true); // skip loop body
                } else {
                    let n = prc_builtin::Integer::from_reader(rdr).unwrap().value; // retrieve number of iterations
                    debug!("{}READI {}", s.i(), n);
                    debug!("{}SIMPLEFOR {}", s.i(), n);
                    // eval next instruction or block N times in a loop
                    for _i in 0..n - 1 {
                        //panic!("EPRCSchema_For not yet implemented!");
                        let mut s_tmp = VmState {
                            indent: s.indent,
                            opstack: s.opstack.clone(),
                            dstack: Default::default(),
                            vars: Default::default(),
                        };
                        self.do_eval(rdr, &mut s_tmp, false);
                        s.merge_from(&mut s_tmp);
                    }
                    if n > 0 {
                        self.do_eval(rdr, s, false);
                    } else {
                        self.do_eval(rdr, s, true);
                    }
                }
            }
            val if val == EPRCSchema_If as u32 => {
                if skip {
                    debug!("{}SKIP IF", s.i());
                    self.do_eval(rdr, s, true); // skip conditional
                    self.do_eval(rdr, s, true); // skip THEN branch
                    if *s.opstack.last().unwrap() == EPRCSchema_Else as u32 {
                        self.do_eval(rdr, s, true); // skip ELSE branch
                    }
                } else {
                    debug!("{}IF", s.i());
                    // eval conditional
                    self.do_eval(rdr, s, false);
                    let conditional = s.dstack.pop().unwrap();
                    let cond_is_true: bool = match conditional {
                        VariableKind::Boolean(b) => b,
                        VariableKind::Double(d) => d != 0.0,
                        VariableKind::Char(c) => c != 0,
                        VariableKind::Integer(i) => i != 0,
                        VariableKind::Unsigned(u) => u != 0,
                        _ => panic!("EPRCSchema_If conditional must be a scalar expression!"),
                    };
                    if cond_is_true {
                        debug!("{}THEN branch", s.i());
                        self.do_eval(rdr, s, false); // evaluate THEN branch
                        if *s.opstack.last().unwrap() == EPRCSchema_Else as u32 {
                            debug!("{}SKIP ELSE branch", s.i());
                            self.do_eval(rdr, s, true); // skip ELSE branch
                        }
                    }
                    // if no - skip then branch and eval else branch
                    else {
                        debug!("{}SKIP THEN branch", s.i());
                        self.do_eval(rdr, s, true); // skip over the THEN branch
                        debug!("{}ELSE branch", s.i());
                        if *s.opstack.last().unwrap() == EPRCSchema_Else as u32 {
                            self.do_eval(rdr, s, false);
                        }
                    }
                }
            }
            val if val == EPRCSchema_Else as u32 => {
                s.indent -= 1;
                if skip {
                    debug!("{}SKIP ELSE", s.i());
                } else {
                    debug!("{}ELSE", s.i());
                }
                s.indent += 1;
                // continue on next token or block
                self.do_eval(rdr, s, skip);
            }
            val if val == EPRCSchema_Value_DeclareAndSet as u32 => {
                let variable_id = s.opstack.pop().unwrap();
                if skip {
                    debug!("{}SKIP DECLARE VAR{}", s.i(), variable_id);
                    self.do_eval(rdr, s, true);
                } else {
                    debug!("{}DECLARE VAR{}", s.i(), variable_id);
                    self.do_eval(rdr, s, false);
                    let variable_value = s.dstack.pop().unwrap();
                    debug!("{}SET VAR{} TO {:?}", s.i(), variable_id, variable_value);
                    s.vars.insert(variable_id, variable_value);
                }
            }
            val if val == EPRCSchema_Value as u32 => {
                let variable_id = s.opstack.pop().unwrap();
                if skip {
                    debug!("{}SKIP READ VAR{}", s.i(), variable_id);
                } else {
                    let variable_value = s.vars[&variable_id].clone();
                    debug!("{}READ VAR{}: {:?}", s.i(), variable_id, variable_value);
                    s.dstack.push(variable_value);
                }
            }
            val if val == EPRCSchema_Value_Constant as u32 => {
                let value = s.opstack.pop().unwrap();
                if !skip {
                    s.dstack.push(VariableKind::Unsigned(value));
                }
            }
            val if val == EPRCSchema_Operator_EQ as u32 => {
                self.do_eval(rdr, s, skip); // eval LHS
                self.do_eval(rdr, s, skip); // eval RHS
                if skip {
                    debug!("{}SKIP EQ", s.i());
                } else {
                    assert!(s.dstack.len() >= 2);
                    let a = s.dstack.pop().unwrap();
                    let b = s.dstack.pop().unwrap();
                    let eq: bool = a == b;
                    debug!("{}EQ {:?} {:?}? -> {}", s.i(), a, b, eq);
                    s.dstack.push(VariableKind::Boolean(eq));
                }
            }
            val if val == EPRCSchema_Block_Start as u32 => {
                if skip {
                    debug!("{}SKIP BLOCK_START", s.i());
                } else {
                    debug!("{}BLOCK_START", s.i());
                }
                while *s.opstack.last().unwrap() != EPRCSchema_Block_End as u32 {
                    self.do_eval(rdr, s, skip);
                }
                self.do_eval(rdr, s, skip); // eval EPRCSchema_Block_End too
            }
            val if val == EPRCSchema_Block_End as u32 => {
                s.indent -= 1;
                if skip {
                    debug!("{}SKIP BLOCK_END", s.i());
                } else {
                    debug!("{}BLOCK_END", s.i());
                }
                s.indent += 1;
            }
            val if val == EPRCSchema_Block_Version as u32 => {
                if skip {
                    debug!("{}SKIP BLOCK_VERSION", s.i());
                    self.do_eval(rdr, s, skip); // version token
                    while *s.opstack.last().unwrap() != EPRCSchema_Block_End as u32 {
                        self.do_eval(rdr, s, skip);
                    }
                    self.do_eval(rdr, s, skip); // eval EPRCSchema_Block_End too
                } else {
                    // version is the next token
                    let version = s.opstack.pop().unwrap();
                    debug!(
                        "{}BLOCK_VERSION {} < {}?",
                        s.i(),
                        self.stored_version,
                        version
                    );
                    if self.stored_version < version {
                        while *s.opstack.last().unwrap() != EPRCSchema_Block_End as u32 {
                            self.do_eval(rdr, s, skip);
                        }
                        self.do_eval(rdr, s, skip); // eval EPRCSchema_Block_End too
                    } else {
                        // skip block
                        while *s.opstack.last().unwrap() != EPRCSchema_Block_End as u32 {
                            self.do_eval(rdr, s, true);
                        }
                        self.do_eval(rdr, s, skip); // eval EPRCSchema_Block_End too
                    }
                }
            }
            val if val == EPRCSchema_ObsoleteToken39NextToIgnore as u32 => {
                // skip/swallow the next token
                s.opstack.pop();
            }
            val if val == EPRCSchema_ObsoleteToken40NextToIgnore as u32 => {
                // skip/swallow the next token
                s.opstack.pop();
            }
            _x => panic!("SchemaEvaluator::do_eval(): Unhandled token: {}!", _x),
        }
        s.indent -= 1;
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prc_builtin::{Boolean, Double, Integer, String, UnsignedInteger};
    use crate::prc_schema::PrcType::*;
    use crate::prc_schema::SchemaTokens::*;
    use bitstream_io::{BigEndian, BitWrite, BitWriter};
    use std::io::Cursor;

    #[test]
    fn dummy() {
        assert_eq!(true, true);
    }

    #[test]
    fn schema_evaluator() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(0);
        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let mut se: SchemaEvaluator = Default::default();
        let _ = se.eval(
            &mut r,
            PrcType::PRC_TYPE_GRAPH_SceneDisplayParameters as u32,
            false,
            0,
        );
    }

    #[test]
    fn prc_schema_basic() {
        #[allow(non_snake_case)]
        let PRCVersion = 8137;
        let mut ops_per_type: HashMap<u32, Vec<u32>> = Default::default();
        #[rustfmt::skip]
        ops_per_type.insert(0, vec![EPRCSchema_Block_Version as u32, PRCVersion-1, EPRCSchema_Data_Integer as u32, EPRCSchema_Data_Integer as u32, EPRCSchema_Block_End as u32]);
        #[rustfmt::skip]
        ops_per_type.insert(1, vec![EPRCSchema_Block_Version as u32, PRCVersion+1, EPRCSchema_Data_Integer as u32, EPRCSchema_Data_Integer as u32, EPRCSchema_Block_End as u32]);
        #[rustfmt::skip]
        ops_per_type.insert(2, vec![EPRCSchema_If as u32, EPRCSchema_Operator_EQ as u32, EPRCSchema_Data_Boolean as u32, EPRCSchema_Value_Constant as u32, 1, EPRCSchema_Data_Double as u32, EPRCSchema_Else as u32, EPRCSchema_Data_String as u32]);
        #[rustfmt::skip]
        ops_per_type.insert(3, vec![EPRCSchema_If as u32, EPRCSchema_Operator_EQ as u32, EPRCSchema_Value_Constant as u32, 1, EPRCSchema_Data_Boolean as u32, EPRCSchema_Block_Start as u32, EPRCSchema_Data_Double as u32, EPRCSchema_Data_Double as u32, EPRCSchema_Block_End as u32, EPRCSchema_Else as u32, EPRCSchema_Block_Start as u32, EPRCSchema_Data_String as u32, EPRCSchema_Data_String as u32, EPRCSchema_Block_End as u32]);
        #[rustfmt::skip]
        ops_per_type.insert(PRC_TYPE_GRAPH_SceneDisplayParameters as u32, vec![19, 39, 1, 20, 8137, 0, 21, 21]);

        let mut bytes = vec![];
        let mut num_trailing_pad_bits = 0;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            // 0

            // 1
            Integer { value: -1 }.to_writer(&mut w).unwrap();
            Integer { value: -2 }.to_writer(&mut w).unwrap();

            // 2
            Boolean { value: true }.to_writer(&mut w).unwrap();
            Double { value: 456.789 }.to_writer(&mut w).unwrap();

            // 2
            Boolean { value: false }.to_writer(&mut w).unwrap();
            String {
                value: std::string::String::from("test-string"),
            }
            .to_writer(&mut w)
            .unwrap();

            // 3
            Boolean { value: true }.to_writer(&mut w).unwrap();
            Double { value: -4.0 }.to_writer(&mut w).unwrap();
            Double { value: 8.0 }.to_writer(&mut w).unwrap();

            // 3
            Boolean { value: false }.to_writer(&mut w).unwrap();
            String {
                value: std::string::String::from("test-string1"),
            }
            .to_writer(&mut w)
            .unwrap();
            String {
                value: std::string::String::from("test-string2"),
            }
            .to_writer(&mut w)
            .unwrap();

            // PRC_TYPE_GRAPH_SceneDisplayParameters
            // nothing needs to be written

            // to start next byte
            Boolean { value: true }.to_writer(&mut w).unwrap();

            // fill partial byte at the end
            while !w.byte_aligned() {
                let _ = w.write_bit(false);
                num_trailing_pad_bits += 1;
            }
        }
        println!("{:?}", num_trailing_pad_bits);
        assert_eq!(53usize, bytes.len());

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let mut se: SchemaEvaluator = Default::default();
        se.stored_version = PRCVersion;
        se.ops_per_type = ops_per_type;
        let mut vars = se.eval(&mut r, 0, false, 0).unwrap();
        assert_eq!(vars.len(), 0usize);

        vars = se.eval(&mut r, 1, false, 0).unwrap();
        assert_eq!(vars.len(), 2usize);
        assert_eq!(vars[0], VariableKind::Integer(-1));
        assert_eq!(vars[1], VariableKind::Integer(-2));

        vars = se.eval(&mut r, 2, false, 0).unwrap();
        assert_eq!(vars.len(), 1usize);
        assert_eq!(vars[0], VariableKind::Double(456.789));

        vars = se.eval(&mut r, 2, false, 0).unwrap();
        assert_eq!(vars.len(), 1usize);
        assert_eq!(vars[0], VariableKind::String("test-string".to_owned()));

        vars = se.eval(&mut r, 3, false, 0).unwrap();
        assert_eq!(vars.len(), 2usize);
        assert_eq!(vars[0], VariableKind::Double(-4.0));
        assert_eq!(vars[1], VariableKind::Double(8.0));

        vars = se.eval(&mut r, 3, false, 0).unwrap();
        assert_eq!(vars.len(), 2usize);
        assert_eq!(vars[0], VariableKind::String("test-string1".to_owned()));
        assert_eq!(vars[1], VariableKind::String("test-string2".to_owned()));

        vars = se
            .eval(
                &mut r,
                PRC_TYPE_GRAPH_SceneDisplayParameters as u32,
                false,
                0,
            )
            .unwrap();
        assert_eq!(vars.len(), 0usize);
        //assert_eq!(vars[0], VariableKind::Boolean(true));
    }

    #[test]
    fn prc_schema_for() {
        #[allow(non_snake_case)]
        let PRCVersion = 8137;
        let mut ops_per_type: HashMap<u32, Vec<u32>> = Default::default(); // from Work-In-Process-WIP-Report.stream-163.prc
        #[rustfmt::skip]
        ops_per_type.insert(PRC_TYPE_MISC_GeneralTransformation as u32, vec![19, 39, 1, 20, 7331, 15, 26, 16, 1, 21, 21]);
        #[rustfmt::skip]
        ops_per_type.insert(4, vec![EPRCSchema_Block_Start as u32, EPRCSchema_ObsoleteToken39NextToIgnore as u32, 1, EPRCSchema_Block_Version as u32, PRCVersion+1, EPRCSchema_For as u32, EPRCSchema_Value_Constant as u32, 16, EPRCSchema_Block_Start as u32, EPRCSchema_Block_Start as u32, EPRCSchema_Data_Double as u32, EPRCSchema_Block_End as u32, EPRCSchema_Block_End as u32, EPRCSchema_Block_End as u32, EPRCSchema_Block_End as u32]);
        #[rustfmt::skip]
        ops_per_type.insert(PRC_TYPE_MKP_View as u32, vec![19, 20, 7309, 0, 21, 20, 8016, 0, 0, 16, 19, 6, 205, 21, 16, 19, 24, 0, 3, 17, 37, 25, 0, 26, 320, 6, 320, 21, 21, 21]);
        #[rustfmt::skip]
        ops_per_type.insert(5, vec![EPRCSchema_Block_Start as u32, EPRCSchema_Block_Version as u32, PRCVersion+1, EPRCSchema_Data_Boolean as u32, 21, 20, 8016, 0, 0, 16, 19, 6, 205, 21, 16, 19, 24, 0, 3, 17, 37, 25, 0, 26, 320, 6, 320, 21, 21, 21]);
        #[rustfmt::skip]
        ops_per_type.insert(6, vec![EPRCSchema_Block_Start as u32, EPRCSchema_Block_Version as u32, PRCVersion+1, EPRCSchema_Data_Boolean as u32, EPRCSchema_Block_End as u32, EPRCSchema_Block_Version as u32, PRCVersion+2, 0, 0, 16, 19, 6, 205, 21, 16, 19, 24, 0, 3, 17, 37, 25, 0, 26, 320, 6, 320, 21, 21, 21]);

        let mut bytes = vec![];
        let mut num_trailing_pad_bits = 0;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            // PRC_TYPE_MISC_GeneralTransformation
            // nothing needs to be written

            // 4
            for i in 0..16 {
                Double {
                    value: i as f64 - 0.5,
                }
                .to_writer(&mut w)
                .unwrap();
            }

            // PRC_TYPE_MKP_View
            // nothing needs to be written

            // 5
            Boolean { value: true }.to_writer(&mut w).unwrap();

            // 6
            Boolean { value: true }.to_writer(&mut w).unwrap();
            Boolean { value: true }.to_writer(&mut w).unwrap();
            Boolean { value: true }.to_writer(&mut w).unwrap();
            Integer { value: 2 }.to_writer(&mut w).unwrap();
            Integer { value: 3 }.to_writer(&mut w).unwrap();
            UnsignedInteger { value: 9 }.to_writer(&mut w).unwrap();
            UnsignedInteger { value: 320 }.to_writer(&mut w).unwrap();
            UnsignedInteger { value: 8 }.to_writer(&mut w).unwrap();

            // fill partial byte at the end
            while !w.byte_aligned() {
                let _ = w.write_bit(false);
                num_trailing_pad_bits += 1;
            }
        }

        println!("{:?}", num_trailing_pad_bits);
        assert_eq!(53usize, bytes.len());

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let mut se: SchemaEvaluator = Default::default();
        se.stored_version = PRCVersion;
        se.ops_per_type = ops_per_type;

        let mut type_id = PRC_TYPE_MISC_GeneralTransformation as u32;
        let mut vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 0usize);

        type_id = 4;
        vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 16usize);
        for i in (0..16).rev() {
            assert_eq!(VariableKind::Double(i as f64 - 0.5), vars[i]);
        }

        type_id = PRC_TYPE_MKP_View as u32;
        vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 0usize);

        type_id = 5;
        vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 1usize);
        assert_eq!(VariableKind::Unsigned(1), vars[0]);

        type_id = 6;
        vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 3/*6*/ as usize);
        assert_eq!(VariableKind::Boolean(true), vars[0]);
        assert_eq!(VariableKind::Boolean(true), vars[1]);
        assert_eq!(VariableKind::Boolean(true), vars[2]);
    }

    #[test]
    fn prc_schema_8995() {
        #[allow(non_snake_case)]
        let PRCVersion = 8137;
        let mut ops_per_type: HashMap<u32, Vec<u32>> = Default::default(); // from Engineering-Data-Release_LandingGear.stream-8995.prc
        #[rustfmt::skip]
        ops_per_type.insert(PRC_TYPE_ROOT_PRCBaseWithGraphics as u32, vec![19, 39, 1, 20, 15083, 17, 37, 26, 1, 4, 6, 802, 21, 21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&(PRC_TYPE_ROOT_PRCBaseWithGraphics as u32)].len(), 14);
        #[rustfmt::skip]
        ops_per_type.insert(303, vec![19,    39,    1,    20,    15083,    3,    6,    801,    21,    21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&303].len(), 10);
        #[rustfmt::skip]
        ops_per_type.insert(501, vec![19, 20, 7309, 0, 21, 20, 8016, 0, 0, 16, 19, 6, 205, 21, 16, 19, 24, 0, 3, 17, 37, 25, 0, 26, 320, 6, 320, 21, 21, 21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&501].len(), 30);
        #[rustfmt::skip]
        ops_per_type.insert(741, vec![19, 39, 1, 20, 8137, 0, 21, 21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&741].len(), 8);
        #[rustfmt::skip]
        ops_per_type.insert(801, vec![19, 39, 1, 20, 15083, 1, 21, 20, 15216, 5, 24, 0, 4, 17, 37, 25, 0, 26, 804, 19, 6, 804, 21, 18, 17, 37, 25, 0, 26, 805, 19, 6, 805, 21, 18, 17, 37, 25, 0, 26, 806, 19, 6, 806, 21, 18, 17, 37, 25, 0, 26, 807, 19, 6, 807, 21, 18, 17, 37, 25, 0, 26, 808, 19, 6, 808, 21, 18, 17, 37, 25, 0, 26, 809, 19, 6, 809, 21, 21, 21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&801].len(), 80);
        #[rustfmt::skip]
        ops_per_type.insert(802, vec![19, 39, 1, 20, 15083, 3, 3, 21, 21]);
        #[rustfmt::skip]
        assert_eq!(ops_per_type[&802].len(), 9);
        #[rustfmt::skip]
        ops_per_type.insert(806, vec![19, 39, 1, 20, 15216, 1, 1, 1, 1, 21, 21]);
        assert_eq!(ops_per_type[&806].len(), 11);

        let mut bytes = vec![];
        let mut num_trailing_pad_bits = 0;
        {
            let mut w = BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
            // 303
            UnsignedInteger { value: 0 }.to_writer(&mut w).unwrap();
            Double { value: 0.5 }.to_writer(&mut w).unwrap();
            String {
                value: "bluff".to_string(),
            }
            .to_writer(&mut w)
            .unwrap();
            Integer { value: 806 }.to_writer(&mut w).unwrap();
            Double { value: 2. }.to_writer(&mut w).unwrap();
            Double { value: 4. }.to_writer(&mut w).unwrap();
            Double { value: 6. }.to_writer(&mut w).unwrap();
            Double { value: 8. }.to_writer(&mut w).unwrap();

            // fill partial byte at the end
            while !w.byte_aligned() {
                let _ = w.write_bit(false);
                num_trailing_pad_bits += 1;
            }
        }

        println!("{:?}", num_trailing_pad_bits);
        assert_eq!(16usize, bytes.len());

        let mut r = BitReader::endian(Cursor::new(&bytes), BigEndian);
        let mut se: SchemaEvaluator = Default::default();
        se.stored_version = PRCVersion;
        se.ops_per_type = ops_per_type;

        let type_id = 303 as u32;
        let vars = se.eval(&mut r, type_id, false, 0).unwrap();
        assert_eq!(vars.len(), 1usize);
        assert_eq!(VariableKind::Double(0.0), vars[0]);
    }
}
