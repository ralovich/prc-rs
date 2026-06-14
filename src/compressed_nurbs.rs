// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use crate::builtin::{sum_up_u, sum_up_v};
use crate::prc_gen::{
    CompressedControlPoints, CompressedKnotVectorU, CompressedKnotVectorV,
    CompressedMultiplicitiesU, CompressedMultiplicitiesV,
};
use crate::vec3::Vec3;
use log::{debug, trace};

#[derive(Debug, Default, Clone)]
pub struct CompressedNurbs {
    pub degree_in_u: u32,
    pub degree_in_v: u32,
    pub number_bits_u: u32,
    pub number_bits_v: u32,
    pub number_ccpt_in_u: u32, // Number_ccpt_in_u = sumOf(mult_u) - degree_in_u - 1
    //Number_ccpt_in_v = sumOf(mult_v) - degree_in_v - 1
    pub number_ccpt_in_v: u32,
    pub number_stored_knots_in_u: u32,
    pub number_stored_knots_in_v: u32,
    pub number_of_knots_in_u: u32,
    pub number_of_knots_in_v: u32,
    is_closed_in_u: bool,
    is_closed_in_v: bool,
    pub number_of_bits_for_isomin: u32, // number of bits used to store first row and column of control points
    pub number_of_bits_for_rest: u32, // number of bits to store the remainder of the control points

    pub tolerance_parameter: f64,  // knots
    pub number_bit_parameter: u32, // knots

    mult_u_flat: Vec<u32>,
    mult_v_flat: Vec<u32>,
    ccpt: Vec<Vec<Vec3>>,
    knots_u_type_param: u32,
    knots_u: Vec<f64>,
    knots_v_type_param: u32,
    knots_v: Vec<f64>,
}

impl CompressedNurbs {
    pub fn set0(&mut self, degree_in_u: u32, degree_in_v: u32, number_stored_knots_in_u: u32) {
        self.degree_in_u = degree_in_u;
        self.number_bits_u = if degree_in_u != 0 {
            ((degree_in_u as f64 + 2.0).ln() / (2.0_f64).ln()).ceil() as u32
        } else {
            2_u32
        };
        self.degree_in_v = degree_in_v;
        self.number_bits_v = if degree_in_v != 0 {
            ((degree_in_v as f64 + 2.0).ln() / (2.0_f64).ln()).ceil() as u32
        } else {
            2_u32
        };
        self.number_stored_knots_in_u = number_stored_knots_in_u;
        self.number_of_knots_in_u = number_stored_knots_in_u - 2;
    }
    pub fn set1(&mut self, mult_u: &Vec<CompressedMultiplicitiesU>, number_stored_knots_in_v: u32) {
        let sum_u;
        (self.mult_u_flat, sum_u) = sum_up_u(mult_u);
        self.number_ccpt_in_u = sum_u - self.degree_in_u - 1;
        self.number_stored_knots_in_v = number_stored_knots_in_v;
        self.number_of_knots_in_v = number_stored_knots_in_v - 2;
    }
    pub fn set2(&mut self, mult_v: &Vec<CompressedMultiplicitiesV>) {
        let sum_v;
        (self.mult_v_flat, sum_v) = sum_up_v(&mult_v);
        self.number_ccpt_in_v = sum_v - self.degree_in_v - 1;
    }
    pub fn set3(
        &mut self,
        is_closed_in_u: bool,
        is_closed_in_v: bool,
        number_of_bits_for_isomin: u32,
        number_of_bits_for_rest: u32,
    ) {
        self.is_closed_in_u = is_closed_in_u;
        self.is_closed_in_v = is_closed_in_v;
        self.number_of_bits_for_isomin = number_of_bits_for_isomin;
        self.number_of_bits_for_rest = number_of_bits_for_rest;
        debug!("{:#?}", self);
    }
    pub fn set4(&mut self, ccpt: &CompressedControlPoints) {
        let mut cp: Vec<Vec<crate::vec3::Vec3>> =
            vec![
                vec![Default::default(); self.number_ccpt_in_v as usize];
                self.number_ccpt_in_u as usize
            ];
        cp[0][0] = crate::vec3::Vec3::new(ccpt.p00.x.value, ccpt.p00.y.value, ccpt.p00.z.value);
        for i in 0..ccpt.ccpt_in_u.len() {
            // FIXME:
            cp[i + 1][0] = crate::vec3::Vec3::new(
                cp[i][0].x() + ccpt.ccpt_in_u[i].x,
                cp[i][0].y() + ccpt.ccpt_in_u[i].y,
                cp[i][0].z() + ccpt.ccpt_in_u[i].z,
            );
        }
        for j in 0..ccpt.ccpt_in_v.len() {
            // FIXME:
            cp[0][j + 1] = Vec3::new(
                cp[0][j].x() + ccpt.ccpt_in_v[j].x,
                cp[0][j].y() + ccpt.ccpt_in_v[j].y,
                cp[0][j].z() + ccpt.ccpt_in_v[j].z,
            );
        }
        fn get_interior_pt(
            ccpt: &CompressedControlPoints,
            nu: usize,
            nv: usize,
            u: usize,
            v: usize,
        ) -> Vec3 {
            assert!(nu > 1);
            assert!(nv > 1);
            let id = (nv - 1) * u + v;
            trace!("id={}", id);
            let inpt = &ccpt.ccpt_interior[id];
            match inpt._type.value {
                0 => Vec3::new(0.0, 0.0, 0.0), // FIXME:
                1 => Vec3::new(0.0, 0.0, inpt.p1z.unwrap().value),
                2 => Vec3::new(inpt.p2x.unwrap().value, inpt.p2y.unwrap().value, 0.0),
                3 => Vec3::new(
                    inpt.p3x.unwrap().value,
                    inpt.p3y.unwrap().value,
                    inpt.p3z.unwrap().value,
                ),
                _ => unreachable!("v={}", inpt._type.value),
            }
        }
        for u in 1..self.number_ccpt_in_u as usize {
            for v in 1..self.number_ccpt_in_v as usize {
                let pt = get_interior_pt(
                    ccpt,
                    self.number_ccpt_in_u as usize,
                    self.number_ccpt_in_v as usize,
                    u - 1,
                    v - 1,
                );
                cp[u][v] = pt;
            }
        }

        //debug!("{:#?}", cp);
        self.ccpt = cp;
    }
    pub fn set5(&mut self, knot_vector_u: &CompressedKnotVectorU) {
        //debug!("{:#?}", knot_vector_u);
        let mut knots = vec![];
        let type_param: u32;
        if !knot_vector_u.is_uniform {
            if knot_vector_u.knots.as_ref().unwrap().is_unknown_form.value {
                type_param = 1;
                for knot in knot_vector_u
                    .knots
                    .as_ref()
                    .unwrap()
                    .compressed_knots
                    .iter()
                {
                    let knot_value;
                    if knot_vector_u
                        .knots
                        .as_ref()
                        .unwrap()
                        .number_bit_parameter
                        .value
                        > 30
                    {
                        knot_value = knot.knot.unwrap().value;
                    } else {
                        knot_value = knot.knot_vbr.unwrap().value;
                    }
                    assert!(knot_value >= 0.0);
                    assert!(knot_value <= 1.0);
                    knots.push(knot_value);
                }
            } else {
                type_param = 2;
            }
        } else {
            type_param = 0;
        }
        debug!("U knots {:#?}", (type_param, &knots));
        self.knots_u_type_param = type_param;
        self.knots_u = knots;
    }
    pub fn set6(&mut self, knot_vector_v: &CompressedKnotVectorV) {
        let mut knots = vec![];
        let type_param: u32;
        if !knot_vector_v.is_uniform {
            if knot_vector_v.knots.as_ref().unwrap().is_unknown_form.value {
                type_param = 1;
                for knot in knot_vector_v
                    .knots
                    .as_ref()
                    .unwrap()
                    .compressed_knots
                    .iter()
                {
                    let knot_value;
                    if knot_vector_v
                        .knots
                        .as_ref()
                        .unwrap()
                        .number_bit_parameter
                        .value
                        > 30
                    {
                        knot_value = knot.knot.unwrap().value;
                    } else {
                        knot_value = knot.knot_vbr.unwrap().value;
                    }
                    assert!(knot_value >= 0.0);
                    assert!(knot_value <= 1.0);
                    knots.push(knot_value);
                }
            } else {
                type_param = 2;
            }
        } else {
            type_param = 0;
        }
        debug!("V knots {:#?}", (type_param, &knots));
        self.knots_v_type_param = type_param;
        self.knots_v = knots;
    }
}
