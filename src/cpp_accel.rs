#[cfg(not(feature = "cpp-accel"))]
fn crc32_fallback(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let lsb = crc & 1;
            crc >>= 1;
            if lsb != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

#[cfg(feature = "cpp-accel")]
unsafe extern "C" {
    fn cpp_crc32(data: *const u8, len: usize) -> u32;
}

pub fn crc32(data: &[u8]) -> u32 {
    #[cfg(feature = "cpp-accel")]
    {
        // SAFETY: `data` points to valid memory for `len` bytes for the duration of this call.
        return unsafe { cpp_crc32(data.as_ptr(), data.len()) };
    }

    #[cfg(not(feature = "cpp-accel"))]
    {
        crc32_fallback(data)
    }
}

pub fn backend_name() -> &'static str {
    #[cfg(feature = "cpp-accel")]
    {
        return "c++";
    }

    #[cfg(not(feature = "cpp-accel"))]
    {
        "rust"
    }
}
