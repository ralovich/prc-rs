// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// SPDX-FileCopyrightText: Copyright Kristóf Ralovich (C) 2025-2026.
// All rights reserved.

use std::env;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    match target_os.as_ref().map(|x| &**x) {
        Ok("windows") => {
            // Command::new("python3")
            //     .args(&["./src/generate_rust", "src/prc.json", "src/prc_gen.rs"])
            //     .status()
            //     .unwrap();
        }
        _ => {
            Command::new("./src/generate_rust")
                .args(&["src/prc.json", "src/prc_gen.rs"])
                .status()
                .unwrap();
            println!("cargo::rerun-if-changed=src/prc.json");
            println!("cargo::rerun-if-changed=src/generate_rust");
            println!("cargo::rerun-if-changed=src/prc_gen.rs");
            println!("cargo::rerun-if-changed=build.rs");
        }
    }
}
