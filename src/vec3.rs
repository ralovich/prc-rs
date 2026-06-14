// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Copy, Clone, Default)]
pub struct Vec3 {
    a: [f64; 3],
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { a: [x, y, z] }
    }
    pub fn x(&self) -> f64 {
        self.a[0]
    }
    pub fn y(&self) -> f64 {
        self.a[1]
    }
    pub fn z(&self) -> f64 {
        self.a[2]
    }
}

impl Display for Vec3 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.a[0], self.a[1], self.a[2])
    }
}
