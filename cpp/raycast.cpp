#include "cpp_accel_common.h"

static inline double cpp_floor(double x) {
    return __builtin_floor(x);
}

extern "C" void cpp_raycast_dda_map(
    double pos_x,
    double pos_y,
    double dir_x,
    double dir_y,
    double max_distance,
    const int32_t* map_data,
    int32_t map_w,
    int32_t map_h,
    DDARaycastResult* out_result) {
    int32_t map_x = static_cast<int32_t>(pos_x);
    int32_t map_y = static_cast<int32_t>(pos_y);

    const double inv_dir_x = (dir_x > 0.00001 || dir_x < -0.00001) ? (1.0 / dir_x) : 1e30;
    const double inv_dir_y = (dir_y > 0.00001 || dir_y < -0.00001) ? (1.0 / dir_y) : 1e30;
    const double delta_dist_x = inv_dir_x > 0.0 ? inv_dir_x : -inv_dir_x;
    const double delta_dist_y = inv_dir_y > 0.0 ? inv_dir_y : -inv_dir_y;

    int32_t step_x;
    int32_t step_y;
    double side_dist_x;
    double side_dist_y;

    if (dir_x < 0.0) {
        step_x = -1;
        side_dist_x = (pos_x - static_cast<double>(map_x)) * delta_dist_x;
    } else {
        step_x = 1;
        side_dist_x = (static_cast<double>(map_x) + 1.0 - pos_x) * delta_dist_x;
    }

    if (dir_y < 0.0) {
        step_y = -1;
        side_dist_y = (pos_y - static_cast<double>(map_y)) * delta_dist_y;
    } else {
        step_y = 1;
        side_dist_y = (static_cast<double>(map_y) + 1.0 - pos_y) * delta_dist_y;
    }

    for (;;) {
        int32_t side;
        double distance;

        if (side_dist_x < side_dist_y) {
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

        if (distance > max_distance) {
            out_result->hit = 0;
            out_result->distance = max_distance;
            out_result->map_x = map_x;
            out_result->map_y = map_y;
            out_result->side = side;
            out_result->wall_x = 0.0;
            return;
        }

        int solid = 1;
        if (map_x >= 0 && map_x < map_w && map_y >= 0 && map_y < map_h) {
            const int32_t idx = map_y * map_w + map_x;
            solid = map_data[idx] > 0;
        }

        if (solid) {
            const double wall_hit = (side == 0) ? (pos_y + distance * dir_y) : (pos_x + distance * dir_x);
            const double wall_floor = cpp_floor(wall_hit);
            double wall_x = wall_hit - wall_floor;
            if (wall_x < 0.0) {
                wall_x += 1.0;
            }

            out_result->hit = 1;
            out_result->distance = distance;
            out_result->map_x = map_x;
            out_result->map_y = map_y;
            out_result->side = side;
            out_result->wall_x = wall_x;
            return;
        }
    }
}
