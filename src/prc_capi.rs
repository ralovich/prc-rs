use crate::common::prc_describe;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prc_parse(
    src_len: u64,
    src: *const i8,
    dst_size: u64,
    dst: *mut i8,
    dst_actual_size: *mut u64,
) -> i32 {
    if src_len < 1 {
        return -1;
    }
    if src == std::ptr::null() {
        return -2;
    }
    if dst_size < 1 {
        return -3;
    }
    if dst == std::ptr::null::<i8>().cast_mut() {
        return -4;
    }
    if dst_actual_size == std::ptr::null::<u64>().cast_mut() {
        return -5;
    }

    let mut bytes = Vec::with_capacity(src_len as usize);
    bytes.resize(src_len as usize, 0);
    for i in 0..src_len as usize {
        bytes[i] = unsafe { *src.add(i) } as u8;
    }

    let verbose: bool = true;
    let all: bool = true;
    let globals: bool = true;
    let tree: bool = true;
    let tess: bool = true;
    let geom: bool = true;
    let extgeom: bool = true;
    let _schema: bool = true;
    let modelfile = true;
    let rv = prc_describe(
        bytes, verbose, all, globals, tree, tess, geom, extgeom, _schema, modelfile,
    );
    match rv {
        Err(_) => {
            unsafe {
                *dst_actual_size = 0;
            }
            return -10;
        }
        Ok(parsed_prc) => {
            // copy resulting text into dst
            let ser = serde_json::to_string(&parsed_prc);
            if ser.is_err() {
                return -11;
            }
            let parsed = ser.unwrap().as_bytes().to_vec();
            if parsed.len() as u64 > dst_size {
                return -12;
            }
            for i in 0..parsed.len() {
                unsafe {
                    *dst.add(i) = parsed[i] as i8;
                }
            }
            unsafe {
                *dst_actual_size = parsed.len() as u64;
            }
            return 0;
        }
    }
}
