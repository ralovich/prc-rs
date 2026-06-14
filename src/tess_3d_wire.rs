// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(non_snake_case)]

use crate::builtin::{Double, Integer};
use crate::constants::Prc3DWireTessFlags;
use log::{debug, warn};

/// See also [prc_rs::prc_gen::PRC_TYPE_TESS_3D_Wire]
#[derive(Default, Clone)]
pub struct Tess3dWire {
    coordinates: Vec<Double>,
    wire_indexes: Vec<Integer>,
    is_segment_color: bool,
    VertexColors_number_of_colors: u32,
}

impl Tess3dWire {
    pub fn set0(&mut self, coordinates: &Vec<Double>, wire_indexes: &Vec<Integer>) {
        self.coordinates = coordinates.clone();
        self.wire_indexes = wire_indexes.clone();
    }
    pub fn set1(&mut self, is_segment_color: bool) {
        self.is_segment_color = is_segment_color;
    }
    /// group___tf3_d_wire_tess_data_____serialize_content2.html
    /// Note that the number of colors is deduced from the number of point indices as calculated from wire_indexes * 3 or 4 (RGB or RGBA).
    /// It is important to remember that implicit points must also have a color (see Special flags for 3DWireTessData tessellation).
    pub fn get_num_vertex_colors(&mut self) -> u32 {
        // TODO
        let mut wires: Vec<Vec<u32>> = vec![];

        if self.wire_indexes.is_empty() {
            // If number_of_wire_indexes is zero, the tessellation is given as a single wire edge containing an array of points as described in SerializeContentBaseTessData.
        } else {
        }

        // if self.TESS_3D_Wire_inside {
        //
        // }

        let mut i = 0;
        while i < self.wire_indexes.len() {
            if self.wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsContinuous as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsContinuous not implemented!");
            }
            if self.wire_indexes[i].value as u32
                & Prc3DWireTessFlags::PRC_3DWIRETESSDATA_IsClosing as u32
                != 0
            {
                warn!("PRC_3DWIRETESSDATA_IsClosing not implemented!");
            }
            // The flag is the leftmost 4 bits and is interpreted using 3D Wire Tess Flags to indicate
            let number_of_indices_per_wire_edge = self.wire_indexes[i].value as u32 & 0x7FFFFFFF;
            debug!(
                "number of indices_per_wire_edge: {}",
                number_of_indices_per_wire_edge
            );
            wires.push(vec![]);

            let start = i + 1;
            for j in 0..number_of_indices_per_wire_edge {
                let id = start + j as usize;
                wires
                    .last_mut()
                    .unwrap()
                    .push(self.wire_indexes[id].value as u32);
                i += 1;
            }

            i += 1;
        }

        self.VertexColors_number_of_colors = 0;
        for w in wires {
            if !self.is_segment_color {
                self.VertexColors_number_of_colors += w.len() as u32;
            } else {
                self.VertexColors_number_of_colors += w.len() as u32 - 1;
            }
        }
        // if self.VertexColors_number_of_colors == 62 {
        //     self.VertexColors_number_of_colors = 60;
        // }
        // if self.VertexColors_number_of_colors == 8 {
        //     self.VertexColors_number_of_colors = 6;
        // }

        debug!(
            "VertexColors_number_of_colors: {}",
            self.VertexColors_number_of_colors
        );
        return self.VertexColors_number_of_colors;
    }
}
