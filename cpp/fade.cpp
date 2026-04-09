#include "cpp_accel_common.h"

extern "C" void cpp_fade_rgba_sub(
    uint8_t* pixels,
    size_t len,
    uint8_t sub_r,
    uint8_t sub_g,
    uint8_t sub_b) {
    size_t i = 0;

    // Unroll by 4 pixels per iteration to reduce branch and index overhead.
    for (; i + 15 < len; i += 16) {
        uint8_t r0 = pixels[i + 0];
        uint8_t g0 = pixels[i + 1];
        uint8_t b0 = pixels[i + 2];
        pixels[i + 0] = (r0 > sub_r) ? static_cast<uint8_t>(r0 - sub_r) : 0;
        pixels[i + 1] = (g0 > sub_g) ? static_cast<uint8_t>(g0 - sub_g) : 0;
        pixels[i + 2] = (b0 > sub_b) ? static_cast<uint8_t>(b0 - sub_b) : 0;
        pixels[i + 3] = 255;

        uint8_t r1 = pixels[i + 4];
        uint8_t g1 = pixels[i + 5];
        uint8_t b1 = pixels[i + 6];
        pixels[i + 4] = (r1 > sub_r) ? static_cast<uint8_t>(r1 - sub_r) : 0;
        pixels[i + 5] = (g1 > sub_g) ? static_cast<uint8_t>(g1 - sub_g) : 0;
        pixels[i + 6] = (b1 > sub_b) ? static_cast<uint8_t>(b1 - sub_b) : 0;
        pixels[i + 7] = 255;

        uint8_t r2 = pixels[i + 8];
        uint8_t g2 = pixels[i + 9];
        uint8_t b2 = pixels[i + 10];
        pixels[i + 8] = (r2 > sub_r) ? static_cast<uint8_t>(r2 - sub_r) : 0;
        pixels[i + 9] = (g2 > sub_g) ? static_cast<uint8_t>(g2 - sub_g) : 0;
        pixels[i + 10] = (b2 > sub_b) ? static_cast<uint8_t>(b2 - sub_b) : 0;
        pixels[i + 11] = 255;

        uint8_t r3 = pixels[i + 12];
        uint8_t g3 = pixels[i + 13];
        uint8_t b3 = pixels[i + 14];
        pixels[i + 12] = (r3 > sub_r) ? static_cast<uint8_t>(r3 - sub_r) : 0;
        pixels[i + 13] = (g3 > sub_g) ? static_cast<uint8_t>(g3 - sub_g) : 0;
        pixels[i + 14] = (b3 > sub_b) ? static_cast<uint8_t>(b3 - sub_b) : 0;
        pixels[i + 15] = 255;
    }

    for (; i + 3 < len; i += 4) {
        uint8_t r = pixels[i + 0];
        uint8_t g = pixels[i + 1];
        uint8_t b = pixels[i + 2];
        pixels[i + 0] = (r > sub_r) ? static_cast<uint8_t>(r - sub_r) : 0;
        pixels[i + 1] = (g > sub_g) ? static_cast<uint8_t>(g - sub_g) : 0;
        pixels[i + 2] = (b > sub_b) ? static_cast<uint8_t>(b - sub_b) : 0;
        pixels[i + 3] = 255;
    }
}
