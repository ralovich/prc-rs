// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

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

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            a: [self.x() + rhs.x(), self.y() + rhs.y(), self.z() + rhs.z()],
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            a: [self.x() - rhs.x(), self.y() - rhs.y(), self.z() - rhs.z()],
        }
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            a: [self.x() * rhs, self.y() * rhs, self.z() * rhs],
        }
    }
}

// consider using https://crates.io/crates/glam
pub fn cross_product(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        a: [
            a.y() * b.z() - a.z() * b.y(),
            a.z() * b.x() - a.x() * b.z(),
            a.x() * b.y() - a.y() * b.x(),
        ],
    }
}
