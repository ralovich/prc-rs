#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate prc;

fuzz_target!(|data: &[u8]| {
    let file_name = "fuzz_target_1.prc".to_owned();
    let opts: prc::common::ParsingOptions = prc::common::ParsingOptions {
        verbose: true,
        all: true,
        globals: true,
        tree: true,
        tess: true,
        geom: true,
        extgeom: true,
        schema: true,
        modelfile: true,
        dump_attachments: false,
    };
    let _ = prc::common::prc_describe(data, &file_name, &opts);
});
