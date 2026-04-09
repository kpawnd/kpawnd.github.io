#[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DDARaycastResult {
    pub hit: u32,
    pub distance: f64,
    pub map_x: i32,
    pub map_y: i32,
    pub side: i32,
    pub wall_x: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CircleWallCollisionResult {
    pub collided: u32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
}

#[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
unsafe extern "C" {
    fn cpp_crc32(data: *const u8, len: usize) -> u32;
    fn cpp_adler32(data: *const u8, len: usize) -> u32;
    fn cpp_fade_rgba_sub(pixels: *mut u8, len: usize, sub_r: u8, sub_g: u8, sub_b: u8);
    fn cpp_raycast_dda_map(
        pos_x: f64,
        pos_y: f64,
        dir_x: f64,
        dir_y: f64,
        max_distance: f64,
        map_data: *const i32,
        map_w: i32,
        map_h: i32,
        out_result: *mut DDARaycastResult,
    );
    fn cpp_circle_wall_collision_step(
        pos_x: f64,
        pos_y: f64,
        vel_x: f64,
        vel_y: f64,
        radius: f64,
        wall_x: i32,
        wall_y: i32,
        out_result: *mut CircleWallCollisionResult,
    );
    fn cpp_b64_encoded_len(input_len: usize) -> usize;
    fn cpp_b64_encode(input: *const u8, input_len: usize, out: *mut u8) -> usize;
    fn cpp_b64_decoded_max_len(input_len: usize) -> usize;
    fn cpp_b64_decode(input: *const u8, input_len: usize, out: *mut u8) -> usize;

    fn cpp_python_new() -> u32;
    fn cpp_python_free(id: u32);
    fn cpp_python_eval(id: u32, code: *const u8, code_len: usize, out: *mut u8, out_cap: usize)
        -> usize;
}

#[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
fn adler32_fallback(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

pub fn crc32(data: &[u8]) -> u32 {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        // SAFETY: `data` points to valid memory for `len` bytes for the duration of this call.
        return unsafe { cpp_crc32(data.as_ptr(), data.len()) };
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        crc32_fallback(data)
    }
}

pub fn adler32(data: &[u8]) -> u32 {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        // SAFETY: `data` points to valid memory for `len` bytes for the duration of this call.
        return unsafe { cpp_adler32(data.as_ptr(), data.len()) };
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        adler32_fallback(data)
    }
}

pub fn fade_rgba_sub(pixels: &mut [u8], sub_r: u8, sub_g: u8, sub_b: u8) {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        // SAFETY: `pixels` is valid mutable memory for `len` bytes.
        unsafe {
            cpp_fade_rgba_sub(
                pixels.as_mut_ptr(),
                pixels.len(),
                sub_r,
                sub_g,
                sub_b,
            )
        };
        return;
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        let mut i = 0usize;
        while i + 3 < pixels.len() {
            pixels[i] = pixels[i].saturating_sub(sub_r);
            pixels[i + 1] = pixels[i + 1].saturating_sub(sub_g);
            pixels[i + 2] = pixels[i + 2].saturating_sub(sub_b);
            pixels[i + 3] = 255;
            i += 4;
        }
    }
}

pub fn raycast_dda_map(
    pos_x: f64,
    pos_y: f64,
    dir_x: f64,
    dir_y: f64,
    max_distance: f64,
    map_data: &[i32],
    map_w: i32,
    map_h: i32,
) -> DDARaycastResult {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        let mut out = DDARaycastResult::default();
        // SAFETY: map_data pointer and out pointer are valid for this FFI call.
        unsafe {
            cpp_raycast_dda_map(
                pos_x,
                pos_y,
                dir_x,
                dir_y,
                max_distance,
                map_data.as_ptr(),
                map_w,
                map_h,
                &mut out as *mut DDARaycastResult,
            );
        }
        return out;
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        let mut map_x = pos_x as i32;
        let mut map_y = pos_y as i32;
        let inv_dir_x = if dir_x.abs() > 0.00001 { 1.0 / dir_x } else { 1e30 };
        let inv_dir_y = if dir_y.abs() > 0.00001 { 1.0 / dir_y } else { 1e30 };
        let delta_dist_x = inv_dir_x.abs();
        let delta_dist_y = inv_dir_y.abs();

        let step_x: i32;
        let step_y: i32;
        let mut side_dist_x: f64;
        let mut side_dist_y: f64;

        if dir_x < 0.0 {
            step_x = -1;
            side_dist_x = (pos_x - map_x as f64) * delta_dist_x;
        } else {
            step_x = 1;
            side_dist_x = (map_x as f64 + 1.0 - pos_x) * delta_dist_x;
        }

        if dir_y < 0.0 {
            step_y = -1;
            side_dist_y = (pos_y - map_y as f64) * delta_dist_y;
        } else {
            step_y = 1;
            side_dist_y = (map_y as f64 + 1.0 - pos_y) * delta_dist_y;
        }

        loop {
            let side: i32;
            let distance: f64;
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                side = 0;
                distance = side_dist_x - delta_dist_x;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                side = 1;
                distance = side_dist_y - delta_dist_y;
            }

            if distance > max_distance {
                return DDARaycastResult {
                    hit: 0,
                    distance: max_distance,
                    map_x,
                    map_y,
                    side,
                    wall_x: 0.0,
                };
            }

            let solid = if map_x >= 0 && map_x < map_w && map_y >= 0 && map_y < map_h {
                let idx = (map_y * map_w + map_x) as usize;
                map_data.get(idx).copied().unwrap_or(1) > 0
            } else {
                true
            };

            if solid {
                let wall_hit = if side == 0 {
                    pos_y + distance * dir_y
                } else {
                    pos_x + distance * dir_x
                };
                let wall_x = wall_hit - wall_hit.floor();
                return DDARaycastResult {
                    hit: 1,
                    distance,
                    map_x,
                    map_y,
                    side,
                    wall_x,
                };
            }
        }
    }
}

pub fn circle_wall_collision_step(
    pos_x: f64,
    pos_y: f64,
    vel_x: f64,
    vel_y: f64,
    radius: f64,
    wall_x: i32,
    wall_y: i32,
) -> CircleWallCollisionResult {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        let mut out = CircleWallCollisionResult::default();
        // SAFETY: output pointer is valid for the duration of the call.
        unsafe {
            cpp_circle_wall_collision_step(
                pos_x,
                pos_y,
                vel_x,
                vel_y,
                radius,
                wall_x,
                wall_y,
                &mut out as *mut CircleWallCollisionResult,
            );
        }
        return out;
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        let wall_min_x = wall_x as f64;
        let wall_min_y = wall_y as f64;
        let wall_max_x = wall_min_x + 1.0;
        let wall_max_y = wall_min_y + 1.0;
        let closest_x = pos_x.max(wall_min_x).min(wall_max_x);
        let closest_y = pos_y.max(wall_min_y).min(wall_max_y);
        let dx = pos_x - closest_x;
        let dy = pos_y - closest_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < radius * radius && dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            let nx = dx / dist;
            let ny = dy / dist;
            let overlap = radius - dist;
            let out_pos_x = pos_x + nx * overlap;
            let out_pos_y = pos_y + ny * overlap;
            let vel_dot = vel_x * nx + vel_y * ny;
            let (out_vel_x, out_vel_y) = if vel_dot < 0.0 {
                (
                    vel_x - (2.0 * vel_dot * 0.5) * nx,
                    vel_y - (2.0 * vel_dot * 0.5) * ny,
                )
            } else {
                (vel_x, vel_y)
            };

            return CircleWallCollisionResult {
                collided: 1,
                pos_x: out_pos_x,
                pos_y: out_pos_y,
                vel_x: out_vel_x,
                vel_y: out_vel_y,
            };
        }

        CircleWallCollisionResult {
            collided: 0,
            pos_x,
            pos_y,
            vel_x,
            vel_y,
        }
    }
}

pub fn b64_encode(data: &[u8]) -> String {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        let out_len = unsafe { cpp_b64_encoded_len(data.len()) };
        let mut out = vec![0u8; out_len];
        // SAFETY: output buffer is preallocated with sufficient size.
        let written = unsafe { cpp_b64_encode(data.as_ptr(), data.len(), out.as_mut_ptr()) };
        out.truncate(written);
        return String::from_utf8_lossy(&out).into_owned();
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        B64.encode(data)
    }
}

pub fn b64_decode(text: &str) -> Result<Vec<u8>, ()> {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        let input = text.as_bytes();
        let max_len = unsafe { cpp_b64_decoded_max_len(input.len()) };
        let mut out = vec![0u8; max_len];
        // SAFETY: output buffer is valid for `max_len` bytes.
        let written = unsafe { cpp_b64_decode(input.as_ptr(), input.len(), out.as_mut_ptr()) };
        if written == usize::MAX {
            return Err(());
        }
        out.truncate(written);
        return Ok(out);
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        B64.decode(text).map_err(|_| ())
    }
}

pub fn backend_name() -> &'static str {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        return "c++";
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        "rust"
    }
}

pub fn python_new() -> u32 {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        return unsafe { cpp_python_new() };
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        0
    }
}

pub fn python_free(id: u32) {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        unsafe { cpp_python_free(id) };
        return;
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        let _ = id;
    }
}

pub fn python_eval(id: u32, code: &str) -> Result<String, String> {
    #[cfg(all(feature = "cpp-accel", not(cpp_accel_disabled)))]
    {
        let mut out = vec![0u8; 2048];
        let written = unsafe { cpp_python_eval(id, code.as_ptr(), code.len(), out.as_mut_ptr(), out.len()) };
        out.truncate(written.min(out.len()));
        if out.is_empty() {
            return Ok(String::new());
        }
        let status = out[0];
        let msg = String::from_utf8_lossy(&out[1..]).into_owned();
        if status == b'0' {
            Ok(msg)
        } else {
            Err(msg)
        }
    }

    #[cfg(any(not(feature = "cpp-accel"), cpp_accel_disabled))]
    {
        let _ = id;
        Ok(code.to_string())
    }
}
