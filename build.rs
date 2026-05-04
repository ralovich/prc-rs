// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use std::process::Command;

fn main() {
    Command::new("./src/generate_rust")
        .args(&["src/prc.json", "src/prc_gen.rs"])
        .status()
        .unwrap();

    println!("cargo::rerun-if-changed=src/prc.json");
    println!("cargo::rerun-if-changed=src/generate_rust");
    println!("cargo::rerun-if-changed=src/prc_gen.rs");
    println!("cargo::rerun-if-changed=build.rs");
}
