#include "cpp_accel_common.h"

static inline double cpp_sqrt(double x) {
    return __builtin_sqrt(x);
}

extern "C" void cpp_circle_wall_collision_step(
    double pos_x,
    double pos_y,
    double vel_x,
    double vel_y,
    double radius,
    int32_t wall_x,
    int32_t wall_y,
    CircleWallCollisionResult* out_result) {
    const double min_x = static_cast<double>(wall_x);
    const double min_y = static_cast<double>(wall_y);
    const double max_x = min_x + 1.0;
    const double max_y = min_y + 1.0;

    const double closest_x = (pos_x < min_x) ? min_x : ((pos_x > max_x) ? max_x : pos_x);
    const double closest_y = (pos_y < min_y) ? min_y : ((pos_y > max_y) ? max_y : pos_y);
    const double dx = pos_x - closest_x;
    const double dy = pos_y - closest_y;
    const double dist_sq = dx * dx + dy * dy;

    if (dist_sq < radius * radius && dist_sq > 0.0001) {
        const double dist = cpp_sqrt(dist_sq);
        const double nx = dx / dist;
        const double ny = dy / dist;
        const double overlap = radius - dist;
        const double out_pos_x = pos_x + nx * overlap;
        const double out_pos_y = pos_y + ny * overlap;
        const double vel_dot = vel_x * nx + vel_y * ny;

        double out_vel_x = vel_x;
        double out_vel_y = vel_y;
        if (vel_dot < 0.0) {
            out_vel_x = vel_x - (2.0 * vel_dot * 0.5) * nx;
            out_vel_y = vel_y - (2.0 * vel_dot * 0.5) * ny;
        }

        out_result->collided = 1;
        out_result->pos_x = out_pos_x;
        out_result->pos_y = out_pos_y;
        out_result->vel_x = out_vel_x;
        out_result->vel_y = out_vel_y;
        return;
    }

    out_result->collided = 0;
    out_result->pos_x = pos_x;
    out_result->pos_y = pos_y;
    out_result->vel_x = vel_x;
    out_result->vel_y = vel_y;
}
