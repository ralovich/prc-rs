// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use prc_rs::*;
//use std::env;
use std::io::*;
use std::path::PathBuf;

// see https://github.com/git/git/blob/v2.52.0/src/varint.rs for ideas

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // globals
    #[arg(long, default_value_t = false)]
    gl: bool,

    // tree
    #[arg(long, default_value_t = false)]
    tr: bool,

    // tessellation
    #[arg(long, default_value_t = false)]
    te: bool,

    // geometry
    #[arg(long, default_value_t = false)]
    ge: bool,

    // extra geometry
    #[arg(long, default_value_t = false)]
    ex: bool,

    // schemas
    #[arg(long, default_value_t = false)]
    sc: bool,

    /// Sets the file to describe
    //#[arg(short, long, value_name = "FILE")]
    fname: PathBuf,
}

fn main() {
    let args = Args::parse();

    let all_sections: bool = args.gl && args.tr && args.te && args.ge && args.ex
        || (!args.gl && !args.tr && !args.te && !args.ge && !args.ex && !args.sc);

    //let args: Vec<String> = env::args().collect();

    // The first argument is the path that was used to call the program.
    //println!("My path is {}.", args[0]);

    // TODO: arg parsing with clap?

    // The rest of the arguments are the passed command line parameters.
    // Call the program like this:
    //   $ ./args arg1 arg2
    //println!("I got {:?} arguments: {:?}.", args.len() - 1, &args[1..]);
    //if args.len() < 2 {
    //    println!("Error: at least a filename is needed!");
    //    std::process::exit(-1);
    //}

    //let fname = &args[args.len() - 1];
    let fname = args.fname.into_os_string().into_string().unwrap();
    //println!("Given filename is {}.", fname);

    #[allow(unused_assignments)]
    let mut verbose: bool = false;
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        //verbose = true;
    }
    verbose = true;

    let _ = common::prc_describe(
        &fname,
        verbose,
        all_sections,
        args.gl,
        args.tr,
        args.te,
        args.ge,
        args.ex,
        args.sc,
    );
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    fn bad_add(a: i32, b: i32) -> i32 {
        a - b
    }

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn test_bad_add() {
        // This assert would fire and test will fail.
        // Please note, that private functions can be tested too!
        assert_ne!(bad_add(1, 2), 3);
    }
}
