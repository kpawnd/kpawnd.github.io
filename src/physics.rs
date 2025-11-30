//! Physics library for game entities
//! Provides collision detection, velocity-based movement, and spatial partitioning

use std::collections::HashMap;

/// 2D Vector for physics calculations
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    #[inline(always)]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    #[inline(always)]
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0001 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self::zero()
        }
    }

    #[inline(always)]
    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    #[inline(always)]
    pub fn scale(&self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
        }
    }

    #[inline(always)]
    pub fn add(&self, other: &Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    #[inline(always)]
    pub fn sub(&self, other: &Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    #[inline(always)]
    pub fn rotate(&self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
        }
    }

    #[inline(always)]
    pub fn perpendicular(&self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    #[inline(always)]
    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    #[inline(always)]
    pub fn distance_to(&self, other: &Self) -> f64 {
        self.sub(other).length()
    }

    #[inline(always)]
    pub fn distance_squared_to(&self, other: &Self) -> f64 {
        self.sub(other).length_squared()
    }
}

/// Axis-Aligned Bounding Box for fast collision detection
#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB {
    #[inline(always)]
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }

    #[inline(always)]
    pub fn from_center_size(center: Vec2, half_width: f64, half_height: f64) -> Self {
        Self {
            min: Vec2::new(center.x - half_width, center.y - half_height),
            max: Vec2::new(center.x + half_width, center.y + half_height),
        }
    }

    #[inline(always)]
    pub fn contains_point(&self, point: &Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    #[inline(always)]
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    #[inline(always)]
    pub fn center(&self) -> Vec2 {
        Vec2::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    #[inline(always)]
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    #[inline(always)]
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
}

/// Circle collider for entities
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f64,
}

impl Circle {
    #[inline(always)]
    pub fn new(x: f64, y: f64, radius: f64) -> Self {
        Self {
            center: Vec2::new(x, y),
            radius,
        }
    }

    #[inline(always)]
    pub fn intersects_circle(&self, other: &Circle) -> bool {
        let dist_sq = self.center.distance_squared_to(&other.center);
        let radii_sum = self.radius + other.radius;
        dist_sq <= radii_sum * radii_sum
    }

    #[inline(always)]
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        // Find closest point on AABB to circle center
        let closest_x = self.center.x.max(aabb.min.x).min(aabb.max.x);
        let closest_y = self.center.y.max(aabb.min.y).min(aabb.max.y);
        let closest = Vec2::new(closest_x, closest_y);
        self.center.distance_squared_to(&closest) <= self.radius * self.radius
    }

    #[inline(always)]
    pub fn contains_point(&self, point: &Vec2) -> bool {
        self.center.distance_squared_to(point) <= self.radius * self.radius
    }

    #[inline(always)]
    pub fn to_aabb(&self) -> AABB {
        AABB::from_center_size(self.center, self.radius, self.radius)
    }
}

/// Physics body with position, velocity, and collision properties
#[derive(Clone, Debug)]
pub struct Body {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub radius: f64,
    pub mass: f64,
    pub friction: f64,
    pub restitution: f64, // Bounciness
    pub is_static: bool,
}

impl Body {
    pub fn new(x: f64, y: f64, radius: f64) -> Self {
        Self {
            position: Vec2::new(x, y),
            velocity: Vec2::zero(),
            acceleration: Vec2::zero(),
            radius,
            mass: 1.0,
            friction: 0.1,
            restitution: 0.3,
            is_static: false,
        }
    }

    pub fn new_static(x: f64, y: f64, radius: f64) -> Self {
        let mut body = Self::new(x, y, radius);
        body.is_static = true;
        body
    }

    #[inline(always)]
    pub fn apply_force(&mut self, force: Vec2) {
        if !self.is_static && self.mass > 0.0 {
            self.acceleration = self.acceleration.add(&force.scale(1.0 / self.mass));
        }
    }

    #[inline(always)]
    pub fn apply_impulse(&mut self, impulse: Vec2) {
        if !self.is_static && self.mass > 0.0 {
            self.velocity = self.velocity.add(&impulse.scale(1.0 / self.mass));
        }
    }

    /// Update position based on velocity (Verlet integration)
    #[inline(always)]
    pub fn integrate(&mut self, dt: f64) {
        if self.is_static {
            return;
        }

        // Apply acceleration to velocity
        self.velocity = self.velocity.add(&self.acceleration.scale(dt));

        // Apply friction
        self.velocity = self.velocity.scale(1.0 - self.friction * dt);

        // Update position
        self.position = self.position.add(&self.velocity.scale(dt));

        // Reset acceleration
        self.acceleration = Vec2::zero();
    }

    #[inline(always)]
    pub fn get_circle(&self) -> Circle {
        Circle::new(self.position.x, self.position.y, self.radius)
    }
}

/// Spatial hash grid for O(1) collision queries
pub struct SpatialGrid {
    inv_cell_size: f64,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f64) -> Self {
        Self {
            inv_cell_size: 1.0 / cell_size,
            cells: HashMap::with_capacity(256),
        }
    }

    #[inline(always)]
    fn hash(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (x * self.inv_cell_size).floor() as i32,
            (y * self.inv_cell_size).floor() as i32,
        )
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert(&mut self, index: usize, position: &Vec2, radius: f64) {
        let min_cell = self.hash(position.x - radius, position.y - radius);
        let max_cell = self.hash(position.x + radius, position.y + radius);

        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                self.cells.entry((cx, cy)).or_default().push(index);
            }
        }
    }

    pub fn query(&self, position: &Vec2, radius: f64) -> Vec<usize> {
        let mut result = Vec::new();
        let min_cell = self.hash(position.x - radius, position.y - radius);
        let max_cell = self.hash(position.x + radius, position.y + radius);

        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                if let Some(indices) = self.cells.get(&(cx, cy)) {
                    for &idx in indices {
                        if !result.contains(&idx) {
                            result.push(idx);
                        }
                    }
                }
            }
        }
        result
    }
}

/// Raycasting for wall detection
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl Ray {
    #[inline(always)]
    pub fn new(origin: Vec2, direction: Vec2) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    #[inline(always)]
    pub fn point_at(&self, t: f64) -> Vec2 {
        self.origin.add(&self.direction.scale(t))
    }
}

/// Result of a raycast
#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub distance: f64,
    pub point: Vec2,
    pub normal: Vec2,
    pub side: i32, // 0 = X side, 1 = Y side
}

/// DDA raycaster for grid-based levels
pub struct DDAResult {
    pub hit: bool,
    pub distance: f64,
    pub map_x: i32,
    pub map_y: i32,
    pub side: i32,
    pub wall_x: f64, // Exact hit position on wall (0.0 - 1.0)
}

/// Perform DDA (Digital Differential Analysis) raycasting on a grid
/// Returns distance to nearest wall and hit information
#[inline(always)]
pub fn raycast_dda<F>(
    pos_x: f64,
    pos_y: f64,
    dir_x: f64,
    dir_y: f64,
    max_distance: f64,
    is_solid: F,
) -> DDAResult
where
    F: Fn(i32, i32) -> bool,
{
    let mut map_x = pos_x as i32;
    let mut map_y = pos_y as i32;

    // Precompute inverse ray directions (avoid division in loop)
    let inv_dir_x = if dir_x.abs() > 0.00001 {
        1.0 / dir_x
    } else {
        1e30
    };
    let inv_dir_y = if dir_y.abs() > 0.00001 {
        1.0 / dir_y
    } else {
        1e30
    };

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

    let mut side: i32;
    let mut distance: f64;

    // DDA loop
    loop {
        // Jump to next grid cell
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

        // Check for wall hit
        if is_solid(map_x, map_y) {
            // Calculate exact wall hit position
            let wall_x = if side == 0 {
                pos_y + distance * dir_y
            } else {
                pos_x + distance * dir_x
            };
            let wall_x = wall_x - wall_x.floor();

            return DDAResult {
                hit: true,
                distance,
                map_x,
                map_y,
                side,
                wall_x,
            };
        }

        // Max distance check
        if distance > max_distance {
            return DDAResult {
                hit: false,
                distance: max_distance,
                map_x,
                map_y,
                side,
                wall_x: 0.0,
            };
        }
    }
}

/// Collision response - separates two overlapping circles
#[inline(always)]
pub fn resolve_circle_collision(a: &mut Body, b: &mut Body) {
    let diff = b.position.sub(&a.position);
    let dist_sq = diff.length_squared();
    let min_dist = a.radius + b.radius;

    if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
        let dist = dist_sq.sqrt();
        let overlap = min_dist - dist;
        let normal = diff.scale(1.0 / dist);

        // Separate bodies
        let total_mass = a.mass + b.mass;
        if !a.is_static && !b.is_static {
            let a_ratio = b.mass / total_mass;
            let b_ratio = a.mass / total_mass;
            a.position = a.position.sub(&normal.scale(overlap * a_ratio));
            b.position = b.position.add(&normal.scale(overlap * b_ratio));
        } else if !a.is_static {
            a.position = a.position.sub(&normal.scale(overlap));
        } else if !b.is_static {
            b.position = b.position.add(&normal.scale(overlap));
        }

        // Calculate collision response (elastic collision)
        if !a.is_static && !b.is_static {
            let rel_vel = b.velocity.sub(&a.velocity);
            let vel_along_normal = rel_vel.dot(&normal);

            if vel_along_normal > 0.0 {
                return; // Moving apart
            }

            let restitution = (a.restitution + b.restitution) * 0.5;
            let j = -(1.0 + restitution) * vel_along_normal;
            let j = j / (1.0 / a.mass + 1.0 / b.mass);

            let impulse = normal.scale(j);
            a.velocity = a.velocity.sub(&impulse.scale(1.0 / a.mass));
            b.velocity = b.velocity.add(&impulse.scale(1.0 / b.mass));
        }
    }
}

/// Check if a circle collides with a grid cell (wall)
#[inline(always)]
pub fn circle_wall_collision(
    pos: &mut Vec2,
    vel: &mut Vec2,
    radius: f64,
    wall_x: i32,
    wall_y: i32,
) -> bool {
    let wall_min = Vec2::new(wall_x as f64, wall_y as f64);
    let wall_max = Vec2::new(wall_x as f64 + 1.0, wall_y as f64 + 1.0);

    // Find closest point on wall to circle
    let closest_x = pos.x.max(wall_min.x).min(wall_max.x);
    let closest_y = pos.y.max(wall_min.y).min(wall_max.y);
    let closest = Vec2::new(closest_x, closest_y);

    let dist_sq = pos.distance_squared_to(&closest);
    if dist_sq < radius * radius && dist_sq > 0.0001 {
        let dist = dist_sq.sqrt();
        let normal = pos.sub(&closest).scale(1.0 / dist);
        let overlap = radius - dist;

        // Push out of wall
        *pos = pos.add(&normal.scale(overlap));

        // Reflect velocity
        let vel_dot = vel.dot(&normal);
        if vel_dot < 0.0 {
            *vel = vel.sub(&normal.scale(2.0 * vel_dot * 0.5)); // 0.5 = friction/bounce
        }

        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_operations() {
        let a = Vec2::new(3.0, 4.0);
        assert!((a.length() - 5.0).abs() < 0.0001);

        let b = Vec2::new(1.0, 0.0);
        let rotated = b.rotate(std::f64::consts::FRAC_PI_2);
        assert!((rotated.x).abs() < 0.0001);
        assert!((rotated.y - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_circle_collision() {
        let c1 = Circle::new(0.0, 0.0, 1.0);
        let c2 = Circle::new(1.5, 0.0, 1.0);
        assert!(c1.intersects_circle(&c2));

        let c3 = Circle::new(3.0, 0.0, 1.0);
        assert!(!c1.intersects_circle(&c3));
    }
}
