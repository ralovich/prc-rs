// -*- mode: rust; coding: utf-8-unix -*-

// SPDX-License-Identifier: MIT
//
// Copyright Kristóf Ralovich (C) 2025-2026. All rights reserved.

use clap::Parser;
use log::{info, warn};
use prc_rs::*;
use std::io::*;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Parse global sections.
    #[arg(long, default_value_t = false)]
    gl: bool,

    /// Parse tree sections.
    #[arg(long, default_value_t = false)]
    tr: bool,

    /// Parse tessellation sections.
    #[arg(long, default_value_t = false)]
    te: bool,

    /// Parse geometry sections.
    #[arg(long, default_value_t = false)]
    ge: bool,

    /// Parse extra geometry sections.
    #[arg(long, default_value_t = false)]
    ex: bool,

    /// Parse schemas.
    #[arg(long, default_value_t = false)]
    sc: bool,

    /// Parse model file section.
    #[arg(long, default_value_t = false)]
    mf: bool,

    /// Sets the file to describe.
    //#[arg(short, long, value_name = "FILE")]
    fname: PathBuf,
}

fn main() -> ExitCode {
    // RUST_LOG=trace cargo run
    pretty_env_logger::init();

    let args = Args::parse();

    let all_sections: bool = args.gl && args.tr && args.te && args.ge && args.ex && args.mf
        || (!args.gl && !args.tr && !args.te && !args.ge && !args.ex && !args.sc && !args.mf);

    let fname = args.fname.into_os_string().into_string().unwrap();

    let mut verbose: bool = false;
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        verbose = true;
    }

    info!(
        "The current time is {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    info!(
        "The current time is {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    match common::prc_describe_file(
        &fname,
        verbose,
        all_sections,
        args.gl,
        args.tr,
        args.te,
        args.ge,
        args.ex,
        args.sc,
        args.mf,
    ) {
        Ok(_data) => ExitCode::SUCCESS,
        Err(why) => {
            warn!("prc_describe_file FAILED: {}", why);
            ExitCode::from(101)
        }
    }
}
