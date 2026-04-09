#pragma once

using size_t = __SIZE_TYPE__;
using uint8_t = __UINT8_TYPE__;
using uint32_t = __UINT32_TYPE__;
using uint64_t = __UINT64_TYPE__;
using int32_t = __INT32_TYPE__;
using int64_t = __INT64_TYPE__;

struct DDARaycastResult {
    uint32_t hit;
    double distance;
    int32_t map_x;
    int32_t map_y;
    int32_t side;
    double wall_x;
};

struct CircleWallCollisionResult {
    uint32_t collided;
    double pos_x;
    double pos_y;
    double vel_x;
    double vel_y;
};
