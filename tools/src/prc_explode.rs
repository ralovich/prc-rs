// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use prc_rs::*;
use std::env;

fn main() {
    // RUST_LOG=trace cargo run
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Error: at least a filename is needed!");
        std::process::exit(-1);
    }

    let fname = &args[args.len() - 1];
    println!("Given filename is {}.", fname);

    let _ = common::prc_explode(fname);
}
