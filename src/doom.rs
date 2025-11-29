use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, HtmlCanvasElement};

use crate::graphics::Graphics;
#[cfg(feature = "webgl")]
use crate::graphics_gl::WebGlGraphics;

// Renderer abstraction: either Canvas2D or WebGL
#[cfg(not(feature = "webgl"))]
type Renderer = Graphics;
#[cfg(feature = "webgl")]
type Renderer = WebGlGraphics;

// Basic raycaster inspired by DOOM/Wolfenstein. All logic in Rust.

const MAP_W: usize = 24;
const MAP_H: usize = 24;
// 1 = wall, 0 = empty. Simple perimeter + some inner walls.
const WORLD_MAP: [i32; MAP_W * MAP_H] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];

#[inline]
fn tile(x: f64, y: f64) -> i32 {
    if x >= 0.0 && y >= 0.0 {
        let xi = x as isize;
        let yi = y as isize;
        if xi >= 0 && yi >= 0 && xi < MAP_W as isize && yi < MAP_H as isize {
            WORLD_MAP[xi as usize + yi as usize * MAP_W]
        } else {
            1
        }
    } else {
        1
    }
}

type LoopClosure = std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>>;
type ResizeClosure = std::cell::RefCell<Option<Closure<dyn FnMut(web_sys::Event)>>>;

thread_local! {
    static GAME: std::cell::RefCell<Option<DoomGame>> = const { std::cell::RefCell::new(None) };
    static GFX: std::cell::RefCell<Option<Renderer>> = const { std::cell::RefCell::new(None) };
    static LOOP: LoopClosure = const { std::cell::RefCell::new(None) };
    static KEYS: std::cell::RefCell<[bool; 256]> = const { std::cell::RefCell::new([false;256]) };
    static MOUSE_DELTA_X: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static MOUSE_CLICKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static RESIZE_CB: ResizeClosure = const { std::cell::RefCell::new(None) };
    static STOPPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Clone)]
struct Monster {
    x: f64,
    y: f64,
    health: i32,
    sprite_type: u8, // 0=imp, 1=demon
    state: MonsterState,
}

#[derive(Clone, Copy, PartialEq)]
enum MonsterState {
    Idle,
    Chasing,
    Dead,
}

struct Projectile {
    x: f64,
    y: f64,
    dir_x: f64,
    dir_y: f64,
    damage: i32,
}

struct DoomGame {
    pos_x: f64,
    pos_y: f64,
    dir_x: f64,
    dir_y: f64,
    plane_x: f64,
    plane_y: f64,
    move_speed: f64,
    rot_speed: f64,
    health: i32,
    ammo: i32,
    current_weapon: u8, // 0=pistol, 1=shotgun
    monsters: Vec<Monster>,
    projectiles: Vec<Projectile>,
    last_shot_time: f64,
    last_spawn_time: f64,
}
impl DoomGame {
    fn new() -> Self {
        // Spawn initial monsters
        let monsters = vec![
            Monster {
                x: 5.0,
                y: 5.0,
                health: 60,
                sprite_type: 0,
                state: MonsterState::Idle,
            },
            Monster {
                x: 18.0,
                y: 5.0,
                health: 80,
                sprite_type: 1,
                state: MonsterState::Idle,
            },
            Monster {
                x: 18.0,
                y: 18.0,
                health: 100,
                sprite_type: 1,
                state: MonsterState::Idle,
            },
        ];

        DoomGame {
            pos_x: 12.0,
            pos_y: 12.0,
            dir_x: -1.0,
            dir_y: 0.0,
            plane_x: 0.0,
            plane_y: 0.66,
            move_speed: 0.08,
            rot_speed: 0.04,
            health: 100,
            ammo: 50,
            current_weapon: 0,
            monsters,
            projectiles: Vec::with_capacity(20),
            last_shot_time: 0.0,
            last_spawn_time: 0.0,
        }
    }

    /// Returns true if the game should stop (ESC pressed)
    fn update(&mut self) -> bool {
        let should_stop = KEYS.with(|k| {
            let keys = k.borrow();
            // W / Up
            if keys[38] || keys[87] {
                let nx = self.pos_x + self.dir_x * self.move_speed;
                let ny = self.pos_y + self.dir_y * self.move_speed;
                if tile(nx, ny) == 0 {
                    self.pos_x = nx;
                    self.pos_y = ny;
                }
            }
            // S / Down
            if keys[40] || keys[83] {
                let nx = self.pos_x - self.dir_x * self.move_speed;
                let ny = self.pos_y - self.dir_y * self.move_speed;
                if tile(nx, ny) == 0 {
                    self.pos_x = nx;
                    self.pos_y = ny;
                }
            }
            // Strafe left (Q or A)
            if keys[65] || keys[81] {
                let perp_x = -self.dir_y;
                let perp_y = self.dir_x;
                let nx = self.pos_x + perp_x * self.move_speed;
                let ny = self.pos_y + perp_y * self.move_speed;
                if tile(nx, ny) == 0 {
                    self.pos_x = nx;
                    self.pos_y = ny;
                }
            }
            // Strafe right (E or D)
            if keys[68] || keys[69] {
                let perp_x = self.dir_y;
                let perp_y = -self.dir_x;
                let nx = self.pos_x + perp_x * self.move_speed;
                let ny = self.pos_y + perp_y * self.move_speed;
                if tile(nx, ny) == 0 {
                    self.pos_x = nx;
                    self.pos_y = ny;
                }
            }
            // Arrow keys for rotation only
            if keys[37] {
                self.rotate(self.rot_speed);
            }
            if keys[39] {
                self.rotate(-self.rot_speed);
            }
            // Switch weapon (1/2 keys)
            if keys[49] {
                self.current_weapon = 0;
            } // 1 = pistol
            if keys[50] && self.ammo >= 2 {
                self.current_weapon = 1;
            } // 2 = shotgun

            // ESC key to exit
            keys[27]
        });

        // Return early if should stop - caller will handle stop_doom() call
        // to avoid RefCell double-borrow (update holds borrow_mut on GAME)
        if should_stop {
            return true;
        }

        // Mouse look
        MOUSE_DELTA_X.with(|md| {
            let dx = md.get();
            if dx.abs() > 0.0 {
                self.rotate(-dx * 0.002);
            }
            md.set(0.0);
        });

        // Shooting (mouse click or space)
        let shoot = KEYS.with(|k| k.borrow()[32])
            || MOUSE_CLICKED.with(|mc| {
                let clicked = mc.get();
                mc.set(false);
                clicked
            });

        let now = js_sys::Date::now();
        if shoot && now - self.last_shot_time > 300.0 && self.ammo > 0 {
            self.shoot();
            self.last_shot_time = now;
        }

        // Update monsters with smarter AI
        for monster in &mut self.monsters {
            if monster.state != MonsterState::Dead {
                let dx = self.pos_x - monster.x;
                let dy = self.pos_y - monster.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 12.0 {
                    monster.state = MonsterState::Chasing;
                    let move_amount = if monster.sprite_type == 1 {
                        0.035
                    } else {
                        0.025
                    }; // Demons faster

                    // Try direct path first
                    let mut nx = monster.x + (dx / dist) * move_amount;
                    let mut ny = monster.y + (dy / dist) * move_amount;

                    // If blocked, try alternative paths (simple obstacle avoidance)
                    if tile(nx, ny) != 0 {
                        // Try moving along X axis only
                        nx = monster.x + (dx / dist.abs()) * move_amount;
                        ny = monster.y;
                        if tile(nx, ny) != 0 {
                            // Try moving along Y axis only
                            nx = monster.x;
                            ny = monster.y + (dy / dist.abs()) * move_amount;
                        }
                    }

                    if tile(nx, ny) == 0 {
                        monster.x = nx;
                        monster.y = ny;
                    }
                } else {
                    monster.state = MonsterState::Idle;
                }
            }
        }

        // Update projectiles
        self.projectiles.retain_mut(|proj| {
            proj.x += proj.dir_x * 0.3;
            proj.y += proj.dir_y * 0.3;

            // Check wall collision
            if tile(proj.x, proj.y) != 0 {
                return false;
            }

            // Check monster collision
            for monster in &mut self.monsters {
                if monster.state != MonsterState::Dead {
                    let dx = proj.x - monster.x;
                    let dy = proj.y - monster.y;
                    if dx * dx + dy * dy < 0.5 {
                        monster.health -= proj.damage;
                        if monster.health <= 0 {
                            monster.state = MonsterState::Dead;
                        }
                        return false;
                    }
                }
            }
            true
        });

        // Infinite monster spawning every 5 seconds
        if now - self.last_spawn_time > 5000.0 {
            self.spawn_monster();
            self.last_spawn_time = now;
        }

        false // Continue running
    }

    fn shoot(&mut self) {
        let cost = if self.current_weapon == 0 { 1 } else { 2 };
        self.ammo -= cost;
        let damage = if self.current_weapon == 0 { 20 } else { 40 };
        self.projectiles.push(Projectile {
            x: self.pos_x,
            y: self.pos_y,
            dir_x: self.dir_x,
            dir_y: self.dir_y,
            damage,
        });
    }

    fn spawn_monster(&mut self) {
        // Find a random spawn point far from player
        let mut x;
        let mut y;
        let mut attempts = 0;

        loop {
            x = 2.0 + (js_sys::Math::random() * (MAP_W - 4) as f64);
            y = 2.0 + (js_sys::Math::random() * (MAP_H - 4) as f64);

            let dx = x - self.pos_x;
            let dy = y - self.pos_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Spawn far from player and in empty space
            if dist > 8.0 && tile(x, y) == 0 {
                break;
            }

            attempts += 1;
            if attempts > 100 {
                return;
            } // Give up if can't find spot
        }

        let sprite_type = if js_sys::Math::random() > 0.5 { 1 } else { 0 };
        let health = if sprite_type == 1 { 80 } else { 60 };

        self.monsters.push(Monster {
            x,
            y,
            health,
            sprite_type,
            state: MonsterState::Idle,
        });
    }

    fn rotate(&mut self, angle: f64) {
        let old_dir_x = self.dir_x;
        self.dir_x = self.dir_x * angle.cos() - self.dir_y * angle.sin();
        self.dir_y = old_dir_x * angle.sin() + self.dir_y * angle.cos();
        let old_plane_x = self.plane_x;
        self.plane_x = self.plane_x * angle.cos() - self.plane_y * angle.sin();
        self.plane_y = old_plane_x * angle.sin() + self.plane_y * angle.cos();
    }

    fn render(&self, gfx: &mut Renderer) {
        gfx.clear(10, 10, 10);
        let w = gfx.width();
        let h = gfx.height();

        // Bail out if dimensions are too small
        if w < 10 || h < 10 {
            let _ = gfx.present();
            return;
        }

        let half_h = h / 2;
        if half_h == 0 {
            let _ = gfx.present();
            return;
        }

        // Draw sky with gradient (top half)
        for y in 0..half_h {
            let sky_ratio = y as f64 / half_h as f64;
            let r = (100.0 + sky_ratio * 40.0) as u8;
            let g = (150.0 + sky_ratio * 50.0) as u8;
            let b = (220.0 - sky_ratio * 20.0) as u8;
            for x in 0..w {
                gfx.set_pixel(x, y, r, g, b);
            }
        }

        // Draw floor gradient (bottom half)
        for y in half_h..h {
            let floor_shade = (20u32 + ((h - y) * 30 / half_h)).min(255) as u8;
            for x in 0..w {
                gfx.set_pixel(x, y, floor_shade / 2, floor_shade / 3, floor_shade / 4);
            }
        }
        for x in 0..w {
            let camera_x = 2.0 * x as f64 / w as f64 - 1.0;
            let ray_dir_x = self.dir_x + self.plane_x * camera_x;
            let ray_dir_y = self.dir_y + self.plane_y * camera_x;

            let mut map_x = self.pos_x as i32;
            let mut map_y = self.pos_y as i32;

            let delta_dist_x = if ray_dir_x == 0.0 {
                1e30
            } else {
                (1.0 / ray_dir_x).abs()
            };
            let delta_dist_y = if ray_dir_y == 0.0 {
                1e30
            } else {
                (1.0 / ray_dir_y).abs()
            };
            let mut side_dist_x;
            let mut side_dist_y;
            let step_x;
            let step_y;

            if ray_dir_x < 0.0 {
                step_x = -1;
                side_dist_x = (self.pos_x - map_x as f64) * delta_dist_x;
            } else {
                step_x = 1;
                side_dist_x = (map_x as f64 + 1.0 - self.pos_x) * delta_dist_x;
            }
            if ray_dir_y < 0.0 {
                step_y = -1;
                side_dist_y = (self.pos_y - map_y as f64) * delta_dist_y;
            } else {
                step_y = 1;
                side_dist_y = (map_y as f64 + 1.0 - self.pos_y) * delta_dist_y;
            }

            let mut hit = 0; // wall hit flag
            let mut side = 0; // NS or EW
            while hit == 0 {
                if side_dist_x < side_dist_y {
                    side_dist_x += delta_dist_x;
                    map_x += step_x;
                    side = 0;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                    side = 1;
                }
                if map_x < 0
                    || map_y < 0
                    || map_x >= MAP_W as i32
                    || map_y >= MAP_H as i32
                    || WORLD_MAP[map_x as usize + map_y as usize * MAP_W] > 0
                {
                    hit = 1;
                }
            }

            // Distance to wall
            let perp_wall_dist = if side == 0 {
                (map_x as f64 - self.pos_x + (1 - step_x) as f64 / 2.0) / ray_dir_x
            } else {
                (map_y as f64 - self.pos_y + (1 - step_y) as f64 / 2.0) / ray_dir_y
            };

            // Guard against bad distances
            if perp_wall_dist <= 0.0 || !perp_wall_dist.is_finite() {
                continue;
            }

            let mut line_height = (h as f64 / perp_wall_dist) as i32;
            if line_height < 0 {
                line_height = 0;
            }
            if line_height > h as i32 * 2 {
                line_height = h as i32 * 2;
            } // cap to prevent overflow
            let draw_start = (-line_height / 2 + h as i32 / 2).max(0);
            let draw_end = (line_height / 2 + h as i32 / 2).min(h as i32 - 1);

            // Skip if nothing to draw
            if draw_end <= draw_start {
                continue;
            }

            // Texture coordinate (simple procedural stripes)
            let mut wall_x = if side == 0 {
                self.pos_y + perp_wall_dist * ray_dir_y
            } else {
                self.pos_x + perp_wall_dist * ray_dir_x
            };
            wall_x -= wall_x.floor();
            let height_diff = (draw_end - draw_start + 1).max(1) as f64;
            for y in draw_start..=draw_end {
                let tex_y_ratio = (y - draw_start) as f64 / height_diff;
                let stripe = ((wall_x * 16.0) as i32) % 2 == 0;
                let (mut r, mut g, mut b) = if stripe {
                    (200, 180, 80)
                } else {
                    (150, 110, 50)
                };
                if side == 1 {
                    r = (r as f64 * 0.7) as u8;
                    g = (g as f64 * 0.7) as u8;
                    b = (b as f64 * 0.7) as u8;
                }
                // Vertical shading by distance
                let fade = (1.0 / (1.0 + perp_wall_dist * 0.2)).min(1.0);
                r = (r as f64 * fade) as u8;
                g = (g as f64 * fade) as u8;
                b = (b as f64 * fade) as u8;
                // Simulate light at center of wall slice
                let light = (1.0 - (tex_y_ratio - 0.5).abs() * 0.8).max(0.2);
                r = (r as f64 * light) as u8;
                g = (g as f64 * light) as u8;
                b = (b as f64 * light) as u8;
                gfx.set_pixel(x, y as u32, r, g, b);
            }
        }

        // Render projectiles as yellow dots
        for proj in &self.projectiles {
            let sprite_x = proj.x - self.pos_x;
            let sprite_y = proj.y - self.pos_y;
            let inv_det = 1.0 / (self.plane_x * self.dir_y - self.dir_x * self.plane_y);
            let transform_x = inv_det * (self.dir_y * sprite_x - self.dir_x * sprite_y);
            let transform_y = inv_det * (-self.plane_y * sprite_x + self.plane_x * sprite_y);

            if transform_y > 0.1 {
                let proj_screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
                let proj_size = ((20.0 / transform_y).abs() as i32).max(2);

                for dx in -proj_size..proj_size {
                    for dy in -proj_size..proj_size {
                        let px = proj_screen_x + dx;
                        let py = (h as i32 / 2) + dy;
                        if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                            gfx.set_pixel(px as u32, py as u32, 255, 255, 0);
                        }
                    }
                }
            }
        }

        // Render sprites (monsters)
        let mut sprites: Vec<(f64, f64, &Monster)> = Vec::with_capacity(self.monsters.len());
        for monster in &self.monsters {
            if monster.state != MonsterState::Dead {
                let sprite_x = monster.x - self.pos_x;
                let sprite_y = monster.y - self.pos_y;

                // Transform sprite to camera space
                let inv_det = 1.0 / (self.plane_x * self.dir_y - self.dir_x * self.plane_y);
                let transform_x = inv_det * (self.dir_y * sprite_x - self.dir_x * sprite_y);
                let transform_y = inv_det * (-self.plane_y * sprite_x + self.plane_x * sprite_y);

                if transform_y > 0.1 {
                    sprites.push((transform_y, transform_x, monster));
                }
            }
        }

        sprites.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (transform_y, transform_x, monster) in sprites {
            if transform_y <= 0.0 || !transform_y.is_finite() {
                continue;
            }
            let sprite_screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
            let sprite_height = ((h as f64 / transform_y).abs()) as i32;
            let sprite_width = sprite_height.min(w as i32 * 2); // cap sprite size

            if sprite_width <= 0 {
                continue;
            }

            let draw_start_y = (-sprite_height / 2 + h as i32 / 2).max(0);
            let draw_end_y = (sprite_height / 2 + h as i32 / 2).min(h as i32 - 1);
            let draw_start_x = (-sprite_width / 2 + sprite_screen_x).max(0);
            let draw_end_x = (sprite_width / 2 + sprite_screen_x).min(w as i32 - 1);

            if draw_end_y <= draw_start_y || draw_end_x <= draw_start_x {
                continue;
            }

            let color = if monster.sprite_type == 0 {
                (200, 50, 50)
            } else {
                (150, 100, 200)
            };
            let height_diff = (draw_end_y - draw_start_y).max(1) as f64;

            for stripe in draw_start_x..draw_end_x {
                if stripe >= 0 && stripe < w as i32 {
                    for y in draw_start_y..draw_end_y {
                        let tex_x = (stripe - (sprite_screen_x - sprite_width / 2)) as f64
                            / sprite_width as f64;
                        let tex_y = (y - draw_start_y) as f64 / height_diff;

                        // Simple sprite pattern (body shape)
                        if tex_x > 0.3 && tex_x < 0.7 && tex_y > 0.2 && tex_y < 0.8 {
                            gfx.set_pixel(stripe as u32, y as u32, color.0, color.1, color.2);
                        }
                    }
                }
            }
        }

        // Draw HUD
        self.draw_hud(gfx);
        let _ = gfx.present();
    }

    fn draw_hud(&self, gfx: &mut Renderer) {
        let w = gfx.width();
        let h = gfx.height();

        // Health bar
        let bar_width = 200;
        let bar_height = 20;
        let bar_x = 20;
        let bar_y = h - 40;

        for x in bar_x..(bar_x + bar_width) {
            for y in bar_y..(bar_y + bar_height) {
                if x < bar_x + (bar_width * self.health as u32 / 100) {
                    gfx.set_pixel(x, y, 200, 0, 0);
                } else {
                    gfx.set_pixel(x, y, 50, 50, 50);
                }
            }
        }

        // Ammo counter
        let ammo_x = w - 100;
        let ammo_y = h - 40;
        self.draw_number(gfx, self.ammo, ammo_x, ammo_y);

        // Weapon indicator
        let weapon_name_x = w / 2 - 50;
        let weapon_name_y = h - 50;
        if self.current_weapon == 0 {
            // Draw "PISTOL" as simple blocks
            for x in weapon_name_x..(weapon_name_x + 60) {
                for dy in 0..5 {
                    gfx.set_pixel(x, weapon_name_y + dy, 255, 255, 0);
                }
            }
        } else {
            // Draw "SHOTGUN" as wider blocks
            for x in weapon_name_x..(weapon_name_x + 80) {
                for dy in 0..5 {
                    gfx.set_pixel(x, weapon_name_y + dy, 255, 165, 0);
                }
            }
        }
    }

    fn draw_number(&self, gfx: &mut Renderer, num: i32, x: u32, y: u32) {
        let digits = num.to_string();
        let mut offset = 0;
        for _ch in digits.chars() {
            // Simple digit rendering (filled rectangles)
            for dx in 0..8 {
                for dy in 0..12 {
                    gfx.set_pixel(x + offset + dx, y + dy, 255, 255, 0);
                }
            }
            offset += 10;
        }
    }
}

#[wasm_bindgen]
pub fn memory_usage() -> String {
    // Access WASM linear memory size
    let mem = wasm_bindgen::memory();
    let buf_val = js_sys::Reflect::get(&mem, &JsValue::from_str("buffer")).ok();
    let bytes = buf_val
        .map(|bv| {
            let ua = js_sys::Uint8Array::new(&bv);
            ua.length() as u64
        })
        .unwrap_or(0);
    let (monsters, projectiles) = GAME.with(|g| {
        if let Some(ref game) = *g.borrow() {
            (game.monsters.len(), game.projectiles.len())
        } else {
            (0, 0)
        }
    });
    format!(
        "wasm_bytes={} monsters={} projectiles={}",
        bytes, monsters, projectiles
    )
}

fn document() -> Document {
    window().unwrap().document().unwrap()
}

fn ensure_canvas(width: u32, height: u32) -> Result<HtmlCanvasElement, JsValue> {
    let doc = document();
    let canvas_el = doc
        .get_element_by_id("game-canvas")
        .ok_or("canvas not found")?;
    let canvas: HtmlCanvasElement = canvas_el.dyn_into()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Ok(canvas)
}

fn update_canvas_size() {
    let w = window().unwrap();
    let width = (w.inner_width().unwrap().as_f64().unwrap() * 0.95) as u32;
    let height = (w.inner_height().unwrap().as_f64().unwrap() * 0.90) as u32;
    GFX.with(|gfx| {
        if let Some(ref mut g) = *gfx.borrow_mut() {
            let _ = g.resize(width, height);
        }
    });
}

fn install_resize_listener() {
    RESIZE_CB.with(|rcb| {
        // Avoid duplicate listeners
        if rcb.borrow().is_some() {
            return;
        }
        let cb = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(|_e: web_sys::Event| {
            // Only resize when graphics is present
            let gfx_present = GFX.with(|g| g.borrow().is_some());
            if gfx_present {
                update_canvas_size();
            }
        }));
        window()
            .unwrap()
            .add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())
            .unwrap();
        *rcb.borrow_mut() = Some(cb);
    });
}

fn uninstall_resize_listener() {
    RESIZE_CB.with(|rcb| {
        if let Some(ref cb) = *rcb.borrow() {
            let _ = window()
                .unwrap()
                .remove_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        }
        *rcb.borrow_mut() = None;
    });
}

fn install_mouse_look(canvas: &HtmlCanvasElement) {
    let move_cb = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(|evt: web_sys::Event| {
        let movement_x = js_sys::Reflect::get(evt.as_ref(), &JsValue::from_str("movementX"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if movement_x.abs() > 0.0 {
            MOUSE_DELTA_X.with(|md| md.set(md.get() + movement_x));
        }
    }));
    canvas
        .add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref())
        .unwrap();
    move_cb.forget();

    // Mouse click to shoot
    let click_cb = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(|_evt: web_sys::Event| {
        MOUSE_CLICKED.with(|mc| mc.set(true));
    }));
    canvas
        .add_event_listener_with_callback("click", click_cb.as_ref().unchecked_ref())
        .unwrap();
    click_cb.forget();
}

fn request_pointer_lock(canvas: &HtmlCanvasElement) {
    canvas.request_pointer_lock();
}

fn start_loop() {
    LOOP.with(|l| {
        if l.borrow().is_some() {
            return;
        }
        let closure = Closure::wrap(Box::new(move |_ts: f64| {
            // If game was stopped, do nothing and avoid rescheduling
            let stopping = STOPPING.with(|s| s.get());
            if stopping {
                return;
            }
            let still_running = GAME.with(|g| g.borrow().is_some());
            if !still_running {
                return;
            }

            // Update phase - get the should_stop flag AFTER borrow is released
            let should_stop = GAME.with(|g| {
                if let Some(ref mut game) = *g.borrow_mut() {
                    game.update()
                } else {
                    false
                }
            });

            // Call stop_doom() OUTSIDE the GAME borrow to avoid RefCell panic
            if should_stop {
                stop_doom();
                return;
            }

            // Render only if game still active after update
            let can_render =
                GAME.with(|g| g.borrow().is_some()) && GFX.with(|gfx| gfx.borrow().is_some());
            if can_render {
                GAME.with(|g| {
                    if let Some(ref mut game) = *g.borrow_mut() {
                        GFX.with(|gfx| {
                            if let Some(ref mut graphics) = *gfx.borrow_mut() {
                                game.render(graphics);
                            }
                        });
                    }
                });
            }
            // Schedule next frame
            let should_continue = GAME.with(|g| g.borrow().is_some());
            let loop_present = LOOP.with(|l2| l2.borrow().is_some());
            if should_continue && loop_present && !STOPPING.with(|s| s.get()) {
                LOOP.with(|l2| {
                    if let Some(ref cb) = *l2.borrow() {
                        let _ = window()
                            .unwrap()
                            .request_animation_frame(cb.as_ref().unchecked_ref());
                    }
                });
            }
        }) as Box<dyn FnMut(f64)>);
        window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();
        *l.borrow_mut() = Some(closure);
    });
}

fn install_key_listeners() {
    let w = window().unwrap();
    let keydown = Closure::<dyn FnMut(_)>::wrap(Box::new(|e: web_sys::KeyboardEvent| {
        KEYS.with(|k| {
            k.borrow_mut()[e.key_code() as usize] = true;
        });
    }));
    let keyup = Closure::<dyn FnMut(_)>::wrap(Box::new(|e: web_sys::KeyboardEvent| {
        KEYS.with(|k| {
            k.borrow_mut()[e.key_code() as usize] = false;
        });
    }));
    w.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    w.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();
    keyup.forget();
}

#[wasm_bindgen]
pub fn start_doom() {
    if let Some(g) = document().get_element_by_id("graphics") {
        g.set_attribute("style", "display:block;").ok();
    }
    if let Some(t) = document().get_element_by_id("terminal") {
        t.set_attribute("style", "display:none;").ok();
    }
    install_key_listeners();
    GFX.with(|gfx| {
        if gfx.borrow().is_none() {
            let w = window().unwrap();
            let width = (w.inner_width().unwrap().as_f64().unwrap() * 0.95) as u32;
            let height = (w.inner_height().unwrap().as_f64().unwrap() * 0.90) as u32;
            let canvas = ensure_canvas(width, height).unwrap();
            install_mouse_look(&canvas);
            request_pointer_lock(&canvas);
            #[cfg(not(feature = "webgl"))]
            {
                let g = Graphics::new("game-canvas", width, height).unwrap();
                *gfx.borrow_mut() = Some(g);
            }
            #[cfg(feature = "webgl")]
            {
                let g = WebGlGraphics::new("game-canvas", width, height).unwrap();
                *gfx.borrow_mut() = Some(g);
            }
        }
    });
    GAME.with(|gm| {
        *gm.borrow_mut() = Some(DoomGame::new());
    });
    update_canvas_size();
    install_resize_listener();
    crate::idle::set_game_active(true);
    crate::idle::set_screensaver_active(false);
    start_loop();
}

#[wasm_bindgen]
pub fn stop_doom() {
    // Stop the loop first
    STOPPING.with(|s| s.set(true));
    LOOP.with(|l| {
        *l.borrow_mut() = None;
    });
    // Detach resize listener to avoid races during teardown
    uninstall_resize_listener();

    // Clear game state
    GAME.with(|gm| {
        *gm.borrow_mut() = None;
    });
    GFX.with(|gfx| {
        *gfx.borrow_mut() = None;
    });
    // Reset input state
    KEYS.with(|k| {
        let mut keys = k.borrow_mut();
        for i in 0..keys.len() {
            keys[i] = false;
        }
    });
    MOUSE_DELTA_X.with(|md| md.set(0.0));
    MOUSE_CLICKED.with(|mc| mc.set(false));

    // Show terminal, hide graphics
    if let Some(g) = document().get_element_by_id("graphics") {
        g.set_attribute("style", "display:none;").ok();
    }
    if let Some(t) = document().get_element_by_id("terminal") {
        t.set_attribute("style", "display:flex;").ok();
    }

    crate::idle::set_game_active(false);
    crate::idle::set_screensaver_active(false);
    STOPPING.with(|s| s.set(false));
}
