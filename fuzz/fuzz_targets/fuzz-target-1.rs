#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate prc;

fuzz_target!(|data: &[u8]| {
    let file_name = "fuzz_target_1.prc".to_owned();
    // fuzzed code goes here
    let _ = prc::common::prc_describe(
        data, &file_name, true, true, true, true, true, true, true, true, true,
    );
});
