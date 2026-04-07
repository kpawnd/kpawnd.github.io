#include <cstddef>
#include <cstdint>

extern "C" uint32_t cpp_crc32(const uint8_t* data, size_t len) {
    uint32_t crc = 0xFFFFFFFFu;

    for (size_t i = 0; i < len; ++i) {
        crc ^= static_cast<uint32_t>(data[i]);
        for (int bit = 0; bit < 8; ++bit) {
            const uint32_t lsb = crc & 1u;
            crc >>= 1u;
            if (lsb) {
                crc ^= 0xEDB88320u;
            }
        }
    }

    return ~crc;
}
