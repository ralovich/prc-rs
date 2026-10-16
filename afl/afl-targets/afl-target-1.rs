#[macro_use]
extern crate afl;
extern crate prc;

fn main() {
    fuzz!(|data: &[u8]| {
        let file_name = "afl-target-1.prc".to_owned();
        let _ = prc::common::prc_describe(
            data, &file_name, true, true, true, true, true, true, true, true, true,
        );
    });
}
