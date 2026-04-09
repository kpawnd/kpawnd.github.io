#include "cpp_accel_common.h"

static const uint8_t B64_ENCODE_TABLE[64] = {
    'A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P',
    'Q','R','S','T','U','V','W','X','Y','Z','a','b','c','d','e','f',
    'g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v',
    'w','x','y','z','0','1','2','3','4','5','6','7','8','9','+','/'
};

static inline int b64_index(uint8_t c) {
    if (c >= 'A' && c <= 'Z') return static_cast<int>(c - 'A');
    if (c >= 'a' && c <= 'z') return static_cast<int>(c - 'a') + 26;
    if (c >= '0' && c <= '9') return static_cast<int>(c - '0') + 52;
    if (c == '+') return 62;
    if (c == '/') return 63;
    return -1;
}

extern "C" size_t cpp_b64_encoded_len(size_t input_len) {
    return ((input_len + 2) / 3) * 4;
}

extern "C" size_t cpp_b64_encode(const uint8_t* input, size_t input_len, uint8_t* out) {
    size_t i = 0;
    size_t o = 0;

    for (; i + 2 < input_len; i += 3) {
        const uint32_t n = (static_cast<uint32_t>(input[i]) << 16) |
                           (static_cast<uint32_t>(input[i + 1]) << 8) |
                           static_cast<uint32_t>(input[i + 2]);
        out[o++] = B64_ENCODE_TABLE[(n >> 18) & 63u];
        out[o++] = B64_ENCODE_TABLE[(n >> 12) & 63u];
        out[o++] = B64_ENCODE_TABLE[(n >> 6) & 63u];
        out[o++] = B64_ENCODE_TABLE[n & 63u];
    }

    if (i < input_len) {
        uint32_t n = static_cast<uint32_t>(input[i]) << 16;
        if (i + 1 < input_len) {
            n |= static_cast<uint32_t>(input[i + 1]) << 8;
        }
        out[o++] = B64_ENCODE_TABLE[(n >> 18) & 63u];
        out[o++] = B64_ENCODE_TABLE[(n >> 12) & 63u];
        if (i + 1 < input_len) {
            out[o++] = B64_ENCODE_TABLE[(n >> 6) & 63u];
            out[o++] = '=';
        } else {
            out[o++] = '=';
            out[o++] = '=';
        }
    }

    return o;
}

extern "C" size_t cpp_b64_decoded_max_len(size_t input_len) {
    return (input_len / 4) * 3 + 3;
}

extern "C" size_t cpp_b64_decode(const uint8_t* input, size_t input_len, uint8_t* out) {
    if ((input_len & 3u) != 0u) {
        return static_cast<size_t>(-1);
    }

    size_t o = 0;
    for (size_t i = 0; i < input_len; i += 4) {
        const uint8_t c0 = input[i + 0];
        const uint8_t c1 = input[i + 1];
        const uint8_t c2 = input[i + 2];
        const uint8_t c3 = input[i + 3];

        const int a = b64_index(c0);
        const int b = b64_index(c1);
        if (a < 0 || b < 0) {
            return static_cast<size_t>(-1);
        }

        const int c = (c2 == '=') ? 0 : b64_index(c2);
        const int d = (c3 == '=') ? 0 : b64_index(c3);
        if ((c2 != '=' && c < 0) || (c3 != '=' && d < 0)) {
            return static_cast<size_t>(-1);
        }

        const uint32_t n = (static_cast<uint32_t>(a) << 18) |
                           (static_cast<uint32_t>(b) << 12) |
                           (static_cast<uint32_t>(c) << 6) |
                           static_cast<uint32_t>(d);

        out[o++] = static_cast<uint8_t>((n >> 16) & 0xFFu);
        if (c2 != '=') {
            out[o++] = static_cast<uint8_t>((n >> 8) & 0xFFu);
        }
        if (c3 != '=') {
            out[o++] = static_cast<uint8_t>(n & 0xFFu);
        }
    }

    return o;
}
