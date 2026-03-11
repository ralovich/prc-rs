use inflate::inflate_bytes_zlib;
use libdeflater::*;

pub fn decompress(section_compressed: &[u8]) -> Result<Vec<u8>, String> {
    let use_slow = true;
    if use_slow {
        let section = inflate_bytes_zlib(&section_compressed)?;
        Ok(section)
    }
    else {
        let decompressed_data = {
            let mut decompressor = Decompressor::new();
            let mut outbuf = Vec::new();
            let mut output_size = section_compressed.len() as usize * 32;
            outbuf.resize(output_size, 0);
            decompressor.zlib_decompress(&section_compressed, &mut outbuf).unwrap();
            // let mut rv = decompressor.zlib_decompress(&section_compressed, &mut outbuf);
            // while rv == Err(DecompressionError::InsufficientSpace) {
            //     output_size *= 2;
            //     outbuf.resize(output_size, 0);
            //     rv = decompressor.zlib_decompress(&section_compressed, &mut outbuf);
            // }
            outbuf
        };
        Ok(decompressed_data)
    }
}