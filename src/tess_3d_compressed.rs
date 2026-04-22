// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

#![allow(non_snake_case)]
#![allow(unused)]

use crate::constants::*;
use crate::function;
use crate::prc_builtin::Boolean;
use crate::prc_builtin::*;
use log::{debug, warn};
use measure_time::debug_time;

#[derive(Default)]
pub struct Tess3dCompressed {
    /// number of faces
    num_faces: u32,
    /// for each face, the list of triangles in that face
    triangles_in_face: Vec<Vec<u32>>,
}
impl Tess3dCompressed {
    pub fn enter(&mut self) {
        self.num_faces = 0;
        self.triangles_in_face.clear();
    }
    pub fn leave(&mut self) {}
    /// reconstruct vertices
    pub fn get_points(
        &self,
        point_array: &Vec<i32>,
        tolerance: f64,
        point_is_reference_array: &Vec<bool>,
        point_reference_array: &Vec<i32>,
        edge_status_array: &Vec<i8>,
        triangle_face_array: &Vec<i32>,
    ) {
        debug_time!("Tess3dCompress::get_points");
        assert_eq!(point_array.len() % 3, 0);
        let mut raw_verts: Vec<[f64; 3]> = Vec::with_capacity(point_array.len() / 3);
        for i in 0..point_array.len() / 3 {
            let x: f64 = point_array[i * 3 + 0] as f64 * tolerance;
            let y: f64 = point_array[i * 3 + 1] as f64 * tolerance;
            let z: f64 = point_array[i * 3 + 2] as f64 * tolerance;
            let vert: [f64; 3] = [x, y, z];
            //dbg!(v0);
            raw_verts.push(vert);
        }
        assert_eq!(raw_verts.len() * 3, point_array.len());

        struct Triangle {
            vertex_ids: [u32; 3],
        }
        let mut tris: Vec<Triangle> = Vec::with_capacity(triangle_face_array.len());

        let mut verts: Vec<[f64; 3]> = Vec::with_capacity(point_array.len() / 3);

        if edge_status_array.len() == triangle_face_array.len() {
            warn!("t3dc: case A");
        } else if edge_status_array.len() == 3 * triangle_face_array.len() {
            warn!("t3dc: case B");
        }

        // for i in 0..point_array.len()/3 {
        //     if i == 0 {
        //         // For V0, its coordinates X,Y,Z are divided by the tolerance and rounded to the nearest integer.
        //         let x: f64 = point_array[i*3+0] as f64 * tolerance;
        //         let y: f64 = point_array[i*3+1] as f64 * tolerance;
        //         let z: f64 = point_array[i*3+2] as f64 * tolerance;
        //
        //         let v0: [f64; 3] = [x, y, z];
        //         dbg!(v0);
        //         verts.push(v0);
        //     }
        //     else if i % 3 == 0 {
        //
        //     }
        //
        //     // V1: DV1 ← V1‐V0
        //     else if i%3 == 1 {
        //         let dx = point_array[i * 3 + 0] as f64 * tolerance;
        //         let dy = point_array[i * 3 + 1] as f64 * tolerance;
        //         let dz = point_array[i * 3 + 2] as f64 * tolerance;
        //         let vi: [f64; 3] = [
        //             verts[(i - 1) as usize][0] + dx,
        //             verts[(i - 1) as usize][1] + dy,
        //             verts[(i - 1) as usize][2] + dz,
        //         ];
        //         verts.push(vi);
        //     }
        //
        //     // TODO:  V2 : DV2 ← V2 ‐ (V0+V1) / 2
        //     else if i%3==2 {
        //         let dx = point_array[i * 3 + 0] as f64 * tolerance;
        //         let dy = point_array[i * 3 + 1] as f64 * tolerance;
        //         let dz = point_array[i * 3 + 2] as f64 * tolerance;
        //         let vi: [f64; 3] = [
        //             (verts[i - 2 as usize][0] + verts[i - 1 as usize][0])/2.0 + dx,
        //             (verts[i - 2 as usize][1] + verts[i - 1 as usize][1])/2.0 + dy,
        //             (verts[i - 2 as usize][2] + verts[i - 1 as usize][2])/2.0 + dz,
        //         ];
        //         verts.push(vi);
        //
        //     }
        // }
        //assert_eq!(verts.len()*3, point_array.len());
    }
    pub fn number_of_reference_points(&self, points_is_reference_array: &Vec<bool>) -> u32 {
        // is the number of non-zero elements in the points_is_reference_array
        let mut num = 0;
        for i in 0..points_is_reference_array.len() {
            if points_is_reference_array[i] {
                num += 1;
            }
        }
        num
    }
    pub fn number_of_triangles(&self, triangle_face_array: &Vec<i32>) -> u32 {
        return triangle_face_array.len() as u32;
    }
    pub fn number_of_faces(&mut self, triangle_face_array: &Vec<i32>) -> u32 {
        debug_time!("Tess3dCompress::number_of_faces");
        if self.num_faces != 0 {
            return self.num_faces as u32;
        }
        if triangle_face_array.is_empty() {
            return 0;
        }
        let min_id = triangle_face_array.into_iter().min().unwrap();
        let max_id = triangle_face_array.into_iter().max().unwrap();
        debug!(
            "TESS_3D_Compressed_number_of_faces: [{}, {}]",
            min_id, max_id
        );
        self.num_faces = *max_id as u32 + 1;
        self.num_faces
    }
    /// triangle_face_array represents, for each triangle, the index of the face to which it belongs
    pub fn number_of_triangles_in_face(
        &mut self,
        triangle_face_array: &Vec<i32>,
        face_id: u32,
    ) -> u32 {
        if !self.triangles_in_face.is_empty() {
            return self.triangles_in_face[face_id as usize].len() as u32;
        }

        self.triangles_in_face
            .resize(self.num_faces as usize, Vec::new());
        for tri_id in 0..triangle_face_array.len() {
            // index of the face this triangle belongs to
            let _face_idx = triangle_face_array[tri_id];
            self.triangles_in_face[_face_idx as usize].push(tri_id as u32);
        }

        let triangles_in_face = self.triangles_in_face[face_id as usize].len() as u32;
        if triangles_in_face == 0 {
            warn!(
                "BUG? number_of_triangles_in_face({}): {}",
                face_id, triangles_in_face
            );
        }
        /*
        println!(
            "{}: face_id: {}, #tris: {}",
            function!(),
            face_id,
            triangles_in_face
        );
         */
        triangles_in_face
    }

    /// see PRC_TYPE_TESS_3D_Compressed.normal_is_reversed
    ///
    /// The number of normals is implicit, depending on the number of triangles and faces.
    /// Vertices have always as many normals as number of faces to which they belong.
    pub fn number_of_normals(
        &mut self,
        triangle_face_array: &Vec<i32>, /*is_face_planar: &Vec<bool>*/
    ) -> u32 {
        debug_time!("TESS_3D_Compressed__number_of_normals");
        //panic!("number_of_normals: Not implemented yet");

        let mut num_normals = 0;
        let mut sum_triangles = 0;
        // for each face
        //   for each triangle
        //     for each vertex
        //let vertex_in_faces: Vec<Vec<u32>> = vec![]; // list of face_ids this vertex belongs to
        let num_faces = self.number_of_faces(triangle_face_array);
        for f in 0..num_faces {
            /*if is_face_planar[f as usize] {
                num_normals += 1;
                continue;
            }*/
            let num_triangles = self.number_of_triangles_in_face(triangle_face_array, f);
            sum_triangles += num_triangles;
            for t in 0..num_triangles {
                num_normals += 1;
            }
        }
        num_normals = triangle_face_array.len() as u32 * 3;
        println!("sum tris: {}", sum_triangles);
        return num_normals;
    }
    /// see PRC_TYPE_TESS_3D_Compressed.is_face_planar
    /// see PRC_TYPE_TESS_3D_Compressed.is_point_color_on_face
    /// see PRC_TYPE_TESS_3D_Compressed.is_multiple_line_attribute_on_face
    /// see PRC_TYPE_TESS_3D_Compressed.face_has_texture
    ///
    /// The size of this array correspond to number of face stored in the mesh.
    /// Is_face_planar is TRUE if corresponding face is planar. A face is a group of triangles. In this case, only one normal is stored for all triangles of this face. It is stored when treating the first vertex of the first triangle of this face.
    pub fn number_of_faces_stored_in_mesh(&mut self, triangle_face_array: &Vec<i32>) -> u32 {
        self.number_of_faces(triangle_face_array)
        //panic!("number_of_faces_stored_in_mesh: Not implemented yet");
        //return triangle_face_array.len() as u32;
    }
}
