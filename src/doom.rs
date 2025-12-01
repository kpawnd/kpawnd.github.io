use js_sys::Reflect;
use std::f64::consts::PI;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    console, MessageEvent, RtcDataChannel, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit,
};
use web_sys::{window, AudioContext, Document, HtmlCanvasElement, OscillatorType};

use crate::graphics::Graphics;
use crate::physics::{circle_wall_collision, raycast_dda, Body, Vec2};

#[cfg(feature = "webgl")]
use crate::graphics_gl::WebGlGraphics;

#[cfg(not(feature = "webgl"))]
type Renderer = Graphics;
#[cfg(feature = "webgl")]
type Renderer = WebGlGraphics;

// Game constants
const MAP_W: usize = 32;
const MAP_H: usize = 32;
const TEX_W: usize = 64;
const TEX_H: usize = 64;

// Number of procedural textures (brick, stone, metal, crate, pillar)
const NUM_TEXTURES: usize = 5;
const MONSTER_TEX_W: usize = 32;
const MONSTER_TEX_H: usize = 32;
const NUM_MONSTER_TEXTURES: usize = 2; // basic + elite

// Simple CC0-style procedural textures (generated at init) stored RGB
static mut TEXTURES: [[u8; TEX_W * TEX_H * 3]; NUM_TEXTURES] =
    [[0u8; TEX_W * TEX_H * 3]; NUM_TEXTURES];
static mut MONSTER_TEXTURES: [[u8; MONSTER_TEX_W * MONSTER_TEX_H * 3]; NUM_MONSTER_TEXTURES] =
    [[0u8; MONSTER_TEX_W * MONSTER_TEX_H * 3]; NUM_MONSTER_TEXTURES];

fn init_textures() {
    // Procedural generation ensures no copyright issues (all math-generated)
    unsafe {
        // 0: Brick wall
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let idx = (y * TEX_W + x) * 3;
                let mortar = y % 16 == 15 || (x % 16 == 15);
                let (r, g, b) = if mortar {
                    (180, 175, 170)
                } else {
                    (150 + (x % 16) as u8, 40, 40)
                };
                TEXTURES[0][idx] = r;
                TEXTURES[0][idx + 1] = g;
                TEXTURES[0][idx + 2] = b;
            }
        }
        // 1: Stone blocks
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let idx = (y * TEX_W + x) * 3;
                let shade = 120 + (((x ^ y) & 15) as u8);
                TEXTURES[1][idx] = shade;
                TEXTURES[1][idx + 1] = shade - 10;
                TEXTURES[1][idx + 2] = shade - 20;
            }
        }
        // 2: Metal panel
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let idx = (y * TEX_W + x) * 3;
                let stripe = (x / 8) % 2 == 0;
                let base = if stripe { 160 } else { 110 };
                TEXTURES[2][idx] = base;
                TEXTURES[2][idx + 1] = base;
                TEXTURES[2][idx + 2] = base + 20;
            }
        }
        // 3: Crate wood
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let idx = (y * TEX_W + x) * 3;
                let grain =
                    ((x as f32 * 0.3).sin() * 10.0) as i32 + ((y as f32 * 0.15).cos() * 8.0) as i32;
                let base = 100 + (grain.clamp(-20, 30)) as u8;
                TEXTURES[3][idx] = base + 30;
                TEXTURES[3][idx + 1] = base + 10;
                TEXTURES[3][idx + 2] = base;
            }
        }
        // 4: Pillar marble
        for y in 0..TEX_H {
            for x in 0..TEX_W {
                let idx = (y * TEX_W + x) * 3;
                let swirl = ((x as f32 * 0.2).sin() + (y as f32 * 0.3).cos()) * 0.5 + 0.5;
                let shade = (200.0 * swirl) as u8;
                TEXTURES[4][idx] = shade;
                TEXTURES[4][idx + 1] = shade;
                TEXTURES[4][idx + 2] = shade - 10;
            }
        }
        // Monster textures
        for y in 0..MONSTER_TEX_H {
            for x in 0..MONSTER_TEX_W {
                let idx = (y * MONSTER_TEX_W + x) * 3;
                // Texture 0: red demon with darker edges
                let edge =
                    !(2..=MONSTER_TEX_W - 3).contains(&x) || !(2..=MONSTER_TEX_H - 3).contains(&y);
                let r = if edge { 120 } else { 200 - (y as u8 / 2) };
                let g = if edge { 20 } else { 40 + (x as u8 / 4) };
                let b = if edge { 20 } else { 30 };
                MONSTER_TEXTURES[0][idx] = r;
                MONSTER_TEXTURES[0][idx + 1] = g;
                MONSTER_TEXTURES[0][idx + 2] = b;

                // Texture 1: elite demon (purple)
                let idx1 = idx;
                let r2 = if edge { 80 } else { 150 + ((x ^ y) & 15) as u8 };
                let g2 = 40 + (y as u8 / 3);
                let b2 = if edge { 120 } else { 200 - (x as u8 / 2) };
                MONSTER_TEXTURES[1][idx1] = r2;
                MONSTER_TEXTURES[1][idx1 + 1] = g2;
                MONSTER_TEXTURES[1][idx1 + 2] = b2;
            }
        }
    }
}

// Difficulty settings
#[derive(Clone, Copy, PartialEq)]
enum Difficulty {
    Easy,   // Monsters deal 5 damage, player has 150 HP
    Normal, // Monsters deal 10 damage, player has 100 HP
    Hard,   // Monsters deal 20 damage, player has 75 HP
}

// Enhanced world map (static mut for runtime initialization)
static mut WORLD_MAP: [i32; MAP_W * MAP_H] = [0; MAP_W * MAP_H];

fn init_world_map() {
    unsafe {
        // Outer walls
        for x in 0..MAP_W {
            WORLD_MAP[x] = 1;
            WORLD_MAP[x + (MAP_H - 1) * MAP_W] = 1;
        }
        for y in 0..MAP_H {
            WORLD_MAP[y * MAP_W] = 1;
            WORLD_MAP[MAP_W - 1 + y * MAP_W] = 1;
        }

        // Inner structures - rooms and corridors
        // Room 1 (top-left)
        for x in 5..10 {
            WORLD_MAP[x + 5 * MAP_W] = 2;
            WORLD_MAP[x + 10 * MAP_W] = 2;
        }
        for y in 5..10 {
            WORLD_MAP[5 + y * MAP_W] = 2;
            WORLD_MAP[10 + y * MAP_W] = 2;
        }
        WORLD_MAP[7 + 10 * MAP_W] = 0; // Door

        // Room 2 (top-right)
        for x in 22..28 {
            WORLD_MAP[x + 5 * MAP_W] = 2;
            WORLD_MAP[x + 10 * MAP_W] = 2;
        }
        for y in 5..10 {
            WORLD_MAP[22 + y * MAP_W] = 2;
            WORLD_MAP[28 + y * MAP_W] = 2;
        }
        WORLD_MAP[25 + 10 * MAP_W] = 0; // Door

        // Room 3 (bottom-left)
        for x in 5..10 {
            WORLD_MAP[x + 22 * MAP_W] = 2;
            WORLD_MAP[x + 27 * MAP_W] = 2;
        }
        for y in 22..27 {
            WORLD_MAP[5 + y * MAP_W] = 2;
            WORLD_MAP[10 + y * MAP_W] = 2;
        }
        WORLD_MAP[7 + 22 * MAP_W] = 0; // Door

        // Room 4 (bottom-right)
        for x in 22..28 {
            WORLD_MAP[x + 22 * MAP_W] = 2;
            WORLD_MAP[x + 27 * MAP_W] = 2;
        }
        for y in 22..27 {
            WORLD_MAP[22 + y * MAP_W] = 2;
            WORLD_MAP[28 + y * MAP_W] = 2;
        }
        WORLD_MAP[25 + 22 * MAP_W] = 0; // Door

        // Central arena with pillars
        for x in 14..18 {
            for y in 14..18 {
                if (x == 15 || x == 16) && (y == 15 || y == 16) {
                    WORLD_MAP[x + y * MAP_W] = 3; // Pillars
                }
            }
        }

        // Scattered crates
        WORLD_MAP[12 + 8 * MAP_W] = 4;
        WORLD_MAP[20 + 8 * MAP_W] = 4;
        WORLD_MAP[12 + 24 * MAP_W] = 4;
        WORLD_MAP[20 + 24 * MAP_W] = 4;
        WORLD_MAP[8 + 16 * MAP_W] = 4;
        WORLD_MAP[24 + 16 * MAP_W] = 4;
    }
}

use std::sync::Mutex;
static ORIGINAL_MAP: Mutex<Option<[i32; MAP_W * MAP_H]>> = Mutex::new(None);

fn backup_original_map() {
    let mut guard = ORIGINAL_MAP.lock().unwrap();
    if guard.is_none() {
        let current = unsafe { WORLD_MAP };
        *guard = Some(current);
    }
}

fn restore_original_map() {
    if let Some(m) = ORIGINAL_MAP.lock().unwrap().take() {
        unsafe {
            WORLD_MAP = m;
        }
    }
}

fn generate_procedural_world() {
    backup_original_map();
    unsafe {
        for i in 0..MAP_W * MAP_H {
            WORLD_MAP[i] = 1;
        }
        for y in 1..MAP_H - 1 {
            for x in 1..MAP_W - 1 {
                WORLD_MAP[x + y * MAP_W] = 0;
            }
        }
        // pillars
        for _ in 0..40 {
            let x = 2 + (js_sys::Math::random() * (MAP_W as f64 - 4.0)) as usize;
            let y = 2 + (js_sys::Math::random() * (MAP_H as f64 - 4.0)) as usize;
            WORLD_MAP[x + y * MAP_W] = 3;
        }
        // room borders with crate texture
        for _ in 0..8 {
            let rw = 4 + (js_sys::Math::random() * 6.0) as usize;
            let rh = 4 + (js_sys::Math::random() * 6.0) as usize;
            let rx = 2 + (js_sys::Math::random() * (MAP_W as f64 - rw as f64 - 4.0)) as usize;
            let ry = 2 + (js_sys::Math::random() * (MAP_H as f64 - rh as f64 - 4.0)) as usize;
            for x in rx..rx + rw {
                WORLD_MAP[x + ry * MAP_W] = 4;
                WORLD_MAP[x + (ry + rh - 1) * MAP_W] = 4;
            }
            for y in ry..ry + rh {
                WORLD_MAP[rx + y * MAP_W] = 4;
                WORLD_MAP[(rx + rw - 1) + y * MAP_W] = 4;
            }
        }
        // clear spawn
        for y in 14..18 {
            for x in 14..18 {
                WORLD_MAP[x + y * MAP_W] = 0;
            }
        }
    }
}

#[inline(always)]
fn tile(x: f64, y: f64) -> i32 {
    if x >= 0.0 && y >= 0.0 {
        let xi = x as usize;
        let yi = y as usize;
        if xi < MAP_W && yi < MAP_H {
            unsafe { WORLD_MAP[xi + yi * MAP_W] }
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
    static AUDIO_CTX: std::cell::RefCell<Option<AudioContext>> = const { std::cell::RefCell::new(None) };
}

fn encode_sdp(s: &str) -> String {
    console::log_1(&JsValue::from_str(&format!(
        "[encode_sdp] Input length: {}, has CRLF: {}",
        s.len(),
        s.contains("\r\n")
    )));
    // Filter out a=max-message-size before encoding - preserve exact line structure
    let lines: Vec<&str> = s.split("\r\n").collect();
    console::log_1(&JsValue::from_str(&format!(
        "[encode_sdp] Split into {} lines",
        lines.len()
    )));
    let filtered: Vec<&str> = lines
        .into_iter()
        .filter(|line| !line.starts_with("a=max-message-size:"))
        .collect();
    console::log_1(&JsValue::from_str(&format!(
        "[encode_sdp] After filter: {} lines",
        filtered.len()
    )));
    let joined = filtered.join("\r\n");
    console::log_1(&JsValue::from_str(&format!(
        "[encode_sdp] Joined length: {}, first 50: {:?}",
        joined.len(),
        &joined.chars().take(50).collect::<String>()
    )));
    window().unwrap().btoa(&joined).unwrap_or_default()
}

fn decode_sdp(s: &str) -> String {
    console::log_1(&JsValue::from_str(&format!(
        "[decode_sdp] Input base64 length: {}",
        s.len()
    )));
    let raw = window().unwrap().atob(s).unwrap_or_default();
    console::log_1(&JsValue::from_str(&format!(
        "[decode_sdp] Decoded length: {}, starts with v=: {}, first 50 chars: {:?}",
        raw.len(),
        raw.starts_with("v="),
        &raw.chars().take(50).collect::<String>()
    )));
    raw
}

#[derive(Clone)]
struct Monster {
    body: Body,
    health: i32,
    max_health: i32,
    sprite_type: u8,
    state: MonsterState,
    attack_cooldown: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum MonsterState {
    Idle,
    Chasing,
    Attacking,
    Dead,
}

struct Projectile {
    body: Body,
    damage: i32,
    lifetime: f64,
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    color: (u8, u8, u8),
    lifetime: f64,
    max_lifetime: f64,
}

struct DoomGame {
    // Player
    player_body: Body,
    dir: Vec2,
    plane: Vec2,
    health: i32,
    max_health: i32,
    ammo: i32,
    current_weapon: u8,

    // Game state
    difficulty: Difficulty,
    score: u32,
    kills: u32,

    // Entities
    monsters: Vec<Monster>,
    projectiles: Vec<Projectile>,
    particles: Vec<Particle>,

    // Timing
    last_shot_time: f64,
    last_spawn_time: f64,
    game_time: f64,

    // Day/night cycle (0.0 = midnight, 0.5 = noon, 1.0 = midnight)
    time_of_day: f64,

    // Ammo pickups
    ammo_pickups: Vec<Vec2>,
    last_ammo_spawn_time: f64,
    remote_players: Vec<RemotePlayer>,
    procedural: bool,
}

impl DoomGame {
    fn new(difficulty: Difficulty) -> Self {
        init_world_map();
        init_textures();

        let (max_health, _damage_mult) = match difficulty {
            Difficulty::Easy => (150, 0.5),
            Difficulty::Normal => (100, 1.0),
            Difficulty::Hard => (75, 2.0),
        };

        let mut player_body = Body::new(16.0, 16.0, 0.3);
        player_body.friction = 0.3;

        let mut monsters = Vec::with_capacity(50);

        // Spawn initial monsters based on difficulty
        let initial_count = match difficulty {
            Difficulty::Easy => 3,
            Difficulty::Normal => 5,
            Difficulty::Hard => 8,
        };

        for i in 0..initial_count {
            let angle = (i as f64 / initial_count as f64) * 2.0 * PI;
            let dist = 8.0;
            let x = 16.0 + angle.cos() * dist;
            let y = 16.0 + angle.sin() * dist;

            if tile(x, y) == 0 {
                monsters.push(Monster::new(x, y, 0, difficulty));
            }
        }

        DoomGame {
            player_body,
            dir: Vec2::new(-1.0, 0.0),
            plane: Vec2::new(0.0, 0.66),
            health: max_health,
            max_health,
            ammo: 50,
            current_weapon: 0,
            difficulty,
            score: 0,
            kills: 0,
            monsters,
            projectiles: Vec::with_capacity(50),
            particles: Vec::with_capacity(100),
            last_shot_time: 0.0,
            last_spawn_time: 0.0,
            game_time: 0.0,
            time_of_day: 0.25, // Start at dawn
            ammo_pickups: Vec::new(),
            last_ammo_spawn_time: 0.0,
            remote_players: Vec::new(),
            procedural: false,
        }
    }

    fn enable_procedural(&mut self) {
        self.procedural = true;
        generate_procedural_world();
    }

    fn update(&mut self, dt: f64) -> bool {
        type ParticleSpawn = (Vec2, Vec2, (u8, u8, u8), f64);

        self.game_time += dt;

        // Day/night cycle (full cycle every 120 seconds)
        self.time_of_day += dt / 120.0;
        if self.time_of_day > 1.0 {
            self.time_of_day -= 1.0;
        }

        // Check for ESC key
        let should_stop = KEYS.with(|k| k.borrow()[27]);
        if should_stop {
            return true;
        }

        // Player movement with physics
        let move_force = 15.0;
        let mut force = Vec2::zero();

        KEYS.with(|k| {
            let keys = k.borrow();
            let forward = self.dir;
            let left = Vec2::new(-self.dir.y, self.dir.x); // left normal
            let right = Vec2::new(self.dir.y, -self.dir.x); // right normal
            let strafe_force = move_force * 0.7;

            if keys[38] || keys[87] {
                force = force.add(&forward.scale(move_force));
            }
            if keys[40] || keys[83] {
                force = force.sub(&forward.scale(move_force));
            }
            if keys[65] || keys[81] {
                force = force.add(&left.scale(strafe_force));
            }
            if keys[68] || keys[69] {
                force = force.add(&right.scale(strafe_force));
            }
            if keys[37] {
                self.rotate(0.05);
            }
            if keys[39] {
                self.rotate(-0.05);
            }
            if keys[49] {
                self.current_weapon = 0;
            }
            if keys[50] && self.ammo >= 2 {
                self.current_weapon = 1;
            }
        });

        // Normalize combined movement to prevent faster diagonal speed
        if force.length() > move_force {
            force = force.normalize().scale(move_force);
        }

        self.player_body.apply_force(force);

        // Spawn ammo pickups periodically if low
        if (self.game_time - self.last_ammo_spawn_time) > 6000.0
            && self.ammo < 100
            && self.ammo_pickups.len() < 6
        {
            // Find a free tile
            for _ in 0..20 {
                let x = 2.0 + js_sys::Math::random() * (MAP_W as f64 - 4.0);
                let y = 2.0 + js_sys::Math::random() * (MAP_H as f64 - 4.0);
                if tile(x, y) == 0 {
                    self.ammo_pickups.push(Vec2::new(x, y));
                    self.last_ammo_spawn_time = self.game_time;
                    break;
                }
            }
        }

        // Pickup collection
        self.ammo_pickups.retain(|p| {
            let dist = self.player_body.position.distance_to(p);
            if dist < 0.6 {
                self.ammo = (self.ammo + 15).min(150);
                false
            } else {
                true
            }
        });

        // Passive ammo trickle if completely dry (avoid soft-lock)
        if self.ammo == 0 && (self.game_time as i32 % 1000) < 16 {
            // roughly every second
            self.ammo += 1;
        }

        // Mouse look
        MOUSE_DELTA_X.with(|md| {
            let dx = md.get();
            if dx.abs() > 0.01 {
                self.rotate(-dx * 0.003);
            }
            md.set(0.0);
        });

        // Shooting
        let shoot = KEYS.with(|k| k.borrow()[32])
            || MOUSE_CLICKED.with(|mc| {
                let clicked = mc.get();
                mc.set(false);
                clicked
            });

        let now = js_sys::Date::now();
        if shoot && now - self.last_shot_time > 250.0 && self.ammo > 0 {
            self.shoot(now);
        }

        // Update player physics with wall collision
        self.player_body.integrate(dt);

        // Wall collision detection using physics system
        let px = self.player_body.position.x as i32;
        let py = self.player_body.position.y as i32;
        let radius = self.player_body.radius;

        // Check surrounding tiles
        for dx in -1..=1 {
            for dy in -1..=1 {
                let tx = px + dx;
                let ty = py + dy;
                if tile(tx as f64, ty as f64) > 0 {
                    circle_wall_collision(
                        &mut self.player_body.position,
                        &mut self.player_body.velocity,
                        radius,
                        tx,
                        ty,
                    );
                }
            }
        }

        // Update monsters with improved AI and physics
        let mut particles_to_spawn: Vec<ParticleSpawn> = Vec::new();

        for monster in &mut self.monsters {
            if monster.state != MonsterState::Dead {
                let to_player = self.player_body.position.sub(&monster.body.position);
                let dist = to_player.length();

                if dist < 15.0 {
                    monster.state = MonsterState::Chasing;

                    if dist < 1.5 && now - monster.attack_cooldown > 1000.0 {
                        // Melee attack
                        monster.state = MonsterState::Attacking;
                        monster.attack_cooldown = now;

                        let damage = match self.difficulty {
                            Difficulty::Easy => 5,
                            Difficulty::Normal => 10,
                            Difficulty::Hard => 20,
                        };

                        // Deal damage directly without extra cooldown
                        self.health -= damage;
                        play_sound(220.0, 0.1); // Hit sound

                        // Collect damage particles
                        for _ in 0..5 {
                            particles_to_spawn.push((
                                self.player_body.position,
                                Vec2::new(
                                    (js_sys::Math::random() - 0.5) * 4.0,
                                    (js_sys::Math::random() - 0.5) * 4.0,
                                ),
                                (255, 0, 0),
                                0.5,
                            ));
                        }
                    } else if dist > 1.5 {
                        // Chase player with pathfinding
                        let dir = to_player.normalize();
                        let speed = if monster.sprite_type == 1 { 3.0 } else { 2.0 };
                        monster.body.apply_force(dir.scale(speed));
                    }
                } else {
                    monster.state = MonsterState::Idle;
                }

                // Monster physics update
                monster.body.integrate(dt);

                // Monster wall collision
                let mx = monster.body.position.x as i32;
                let my = monster.body.position.y as i32;
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let tx = mx + dx;
                        let ty = my + dy;
                        if tile(tx as f64, ty as f64) > 0 {
                            circle_wall_collision(
                                &mut monster.body.position,
                                &mut monster.body.velocity,
                                monster.body.radius,
                                tx,
                                ty,
                            );
                        }
                    }
                }
            }
        }

        // Spawn collected particles
        for (pos, vel, color, lifetime) in particles_to_spawn {
            self.spawn_particle(pos, vel, color, lifetime);
        }

        // Update projectiles with physics
        let mut more_particles: Vec<ParticleSpawn> = Vec::new();

        let projectiles_to_check: Vec<_> = self
            .projectiles
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.body.position, p.damage))
            .collect();

        self.projectiles.retain_mut(|proj| {
            proj.body.integrate(dt);
            proj.lifetime -= dt;

            if proj.lifetime <= 0.0 {
                return false;
            }

            // Wall collision
            let px = proj.body.position.x as i32;
            let py = proj.body.position.y as i32;
            if tile(px as f64, py as f64) > 0 {
                // Collect impact particles
                for _ in 0..3 {
                    more_particles.push((
                        proj.body.position,
                        Vec2::new(
                            (js_sys::Math::random() - 0.5) * 2.0,
                            (js_sys::Math::random() - 0.5) * 2.0,
                        ),
                        (255, 255, 100),
                        0.3,
                    ));
                }
                return false;
            }

            true
        });

        // Check monster collisions separately
        for (idx, proj_pos, damage) in projectiles_to_check {
            if idx >= self.projectiles.len() {
                continue;
            }

            for monster in self.monsters.iter_mut() {
                if monster.state != MonsterState::Dead {
                    let dist = proj_pos.distance_to(&monster.body.position);
                    if dist < 0.5 {
                        monster.health -= damage;
                        if monster.health <= 0 {
                            monster.state = MonsterState::Dead;
                            self.score += 100;
                            self.kills += 1;
                            play_sound(150.0, 0.2); // Death sound

                            // Collect death particles
                            for _ in 0..10 {
                                more_particles.push((
                                    monster.body.position,
                                    Vec2::new(
                                        (js_sys::Math::random() - 0.5) * 5.0,
                                        (js_sys::Math::random() - 0.5) * 5.0,
                                    ),
                                    if monster.sprite_type == 0 {
                                        (200, 50, 50)
                                    } else {
                                        (150, 100, 200)
                                    },
                                    1.0,
                                ));
                            }
                        }
                        // Mark projectile for removal (we'll clean up by index)
                        if idx < self.projectiles.len() {
                            self.projectiles[idx].lifetime = 0.0;
                        }
                        break;
                    }
                }
            }
        }

        // Remove dead projectiles
        self.projectiles.retain(|p| p.lifetime > 0.0);

        // Spawn all collected particles
        for (pos, vel, color, lifetime) in more_particles {
            self.spawn_particle(pos, vel, color, lifetime);
        }

        // Update particles
        self.particles.retain_mut(|p| {
            p.lifetime -= dt;
            if p.lifetime <= 0.0 {
                return false;
            }

            p.velocity = p.velocity.scale(0.95); // Air resistance
            p.position = p.position.add(&p.velocity.scale(dt));
            true
        });

        // Remove dead monsters occasionally to prevent lag
        if self.monsters.len() > 50 {
            self.monsters.retain(|m| m.state != MonsterState::Dead);
        }

        // Spawn new monsters
        if now - self.last_spawn_time > 8000.0 && self.monsters.len() < 50 {
            self.spawn_monster(now);
        }

        // Game over check
        if self.health <= 0 {
            return true;
        }

        false
    }

    fn shoot(&mut self, now: f64) {
        // Pistol (weapon 0) now infinite ammo (cost 0) so player never hard locks.
        let cost = if self.current_weapon == 0 { 0 } else { 2 };
        self.ammo -= cost;
        self.last_shot_time = now;

        let damage = if self.current_weapon == 0 { 25 } else { 50 };

        // Shoot sound
        play_sound(
            if self.current_weapon == 0 {
                440.0
            } else {
                330.0
            },
            0.05,
        );

        // Create projectile with physics
        let mut proj_body = Body::new(
            self.player_body.position.x,
            self.player_body.position.y,
            0.1,
        );
        proj_body.velocity = self.dir.scale(20.0);
        proj_body.friction = 0.0;

        self.projectiles.push(Projectile {
            body: proj_body,
            damage,
            lifetime: 5.0,
        });

        // Muzzle flash particles
        for _ in 0..5 {
            self.spawn_particle(
                self.player_body.position.add(&self.dir.scale(0.5)),
                self.dir.scale(2.0).add(&Vec2::new(
                    (js_sys::Math::random() - 0.5) * 1.0,
                    (js_sys::Math::random() - 0.5) * 1.0,
                )),
                (255, 200, 0),
                0.2,
            );
        }
    }

    fn spawn_monster(&mut self, now: f64) {
        for _ in 0..10 {
            let x = 2.0 + js_sys::Math::random() * (MAP_W - 4) as f64;
            let y = 2.0 + js_sys::Math::random() * (MAP_H - 4) as f64;

            let dist = self.player_body.position.distance_to(&Vec2::new(x, y));
            if dist > 10.0 && tile(x, y) == 0 {
                let sprite_type = if js_sys::Math::random() > 0.6 { 1 } else { 0 };

                self.monsters
                    .push(Monster::new(x, y, sprite_type, self.difficulty));
                self.last_spawn_time = now;
                break;
            }
        }
    }

    fn spawn_particle(&mut self, pos: Vec2, vel: Vec2, color: (u8, u8, u8), lifetime: f64) {
        if self.particles.len() < 100 {
            self.particles.push(Particle {
                position: pos,
                velocity: vel,
                color,
                lifetime,
                max_lifetime: lifetime,
            });
        }
    }

    fn rotate(&mut self, angle: f64) {
        self.dir = self.dir.rotate(angle);
        self.plane = self.plane.rotate(angle);
    }

    fn render(&self, gfx: &mut Renderer) {
        let w = gfx.width();
        let h = gfx.height();

        if w < 10 || h < 10 {
            gfx.clear(0, 0, 0);
            let _ = gfx.present();
            return;
        }

        let half_h = h / 2;

        // Sky color based on time of day
        let (sky_r, sky_g, sky_b) = self.get_sky_color();

        // Draw sky with fast horizontal lines (huge performance boost)
        for y in 0..half_h {
            let gradient = y as f32 / half_h as f32;
            let r = (sky_r as f32 * (1.0 - gradient * 0.3)) as u8;
            let g = (sky_g as f32 * (1.0 - gradient * 0.3)) as u8;
            let b = (sky_b as f32 * (1.0 - gradient * 0.2)) as u8;
            gfx.draw_hline(0, w - 1, y, r, g, b);
        }

        // Draw stars at night (reduced for performance)
        if self.time_of_day < 0.2 || self.time_of_day > 0.8 {
            let star_brightness = if self.time_of_day < 0.2 {
                (0.2 - self.time_of_day) * 5.0
            } else {
                (self.time_of_day - 0.8) * 5.0
            };

            for i in 0..30 {
                let sx = ((i * 73) % w as i32) as u32;
                let sy = ((i * 37) % half_h as i32) as u32;
                let brightness = (255.0 * star_brightness.min(1.0)) as u8;
                if sx < w && sy < half_h {
                    gfx.set_pixel_rgb(sx, sy, brightness, brightness, brightness);
                }
            }
        }

        // Draw floor with fast horizontal lines
        for y in half_h..h {
            let shade = (30 + (h - y) * 20 / half_h).min(255) as u8;
            gfx.draw_hline(0, w - 1, y, shade / 3, shade / 4, shade / 5);
        } // Z-buffer for sprite rendering
        let mut z_buffer = vec![f64::MAX; w as usize];

        // Raycast walls (optimized)
        for x in 0..w {
            let camera_x = 2.0 * x as f64 / w as f64 - 1.0;
            let ray_dir = Vec2::new(
                self.dir.x + self.plane.x * camera_x,
                self.dir.y + self.plane.y * camera_x,
            );

            let result = raycast_dda(
                self.player_body.position.x,
                self.player_body.position.y,
                ray_dir.x,
                ray_dir.y,
                50.0,
                |mx, my| tile(mx as f64, my as f64) > 0,
            );

            if !result.hit || result.distance <= 0.0 {
                continue;
            }

            z_buffer[x as usize] = result.distance;

            let line_height = (h as f64 / result.distance).min(h as f64 * 2.0) as u32;
            let draw_start = (half_h as i32 - line_height as i32 / 2).max(0) as u32;
            let draw_end = (half_h + line_height / 2).min(h);

            if draw_end <= draw_start {
                continue;
            }

            // Wall color based on type
            let wall_type = tile(result.map_x as f64, result.map_y as f64);
            let tex_index = match wall_type {
                2 => 1, // stone
                3 => 4, // pillar marble
                4 => 3, // crate
                _ => 0, // brick default
            };
            let tex_x = ((result.wall_x * TEX_W as f64) as i32 & (TEX_W as i32 - 1)) as usize;
            let side_mult = if result.side == 1 { 0.65 } else { 1.0 };
            let fog = (1.0 / (1.0 + result.distance * 0.18)).min(1.0) as f32;
            let day_light = self.get_ambient_light();
            unsafe {
                for sy in draw_start..draw_end {
                    let d_y = sy - draw_start;
                    let tex_y = ((d_y as f64 / (draw_end - draw_start) as f64) * TEX_H as f64)
                        as usize
                        & (TEX_H - 1);
                    let base = (tex_y * TEX_W + tex_x) * 3;
                    let mut r = TEXTURES[tex_index][base] as f32;
                    let mut g = TEXTURES[tex_index][base + 1] as f32;
                    let mut b = TEXTURES[tex_index][base + 2] as f32;
                    // Lighting & fog
                    r = r * side_mult * fog * day_light;
                    g = g * side_mult * fog * day_light;
                    b = b * side_mult * fog * day_light;
                    gfx.set_pixel_rgb(x, sy, r as u8, g as u8, b as u8);
                }
            }
        }

        // Render ammo pickups (simple blue squares)
        for ap in &self.ammo_pickups {
            let sprite_pos = ap.sub(&self.player_body.position);
            let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
            let transform_x = inv_det * (self.dir.y * sprite_pos.x - self.dir.x * sprite_pos.y);
            let transform_y =
                inv_det * (-self.plane.y * sprite_pos.x + self.plane.x * sprite_pos.y);
            if transform_y > 0.1 && transform_y < 20.0 {
                let screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
                let size = ((10.0 / transform_y).abs() as i32).clamp(2, 12);
                if screen_x >= 0 && screen_x < w as i32 {
                    for dy in -size..=size {
                        for dx in -size..=size {
                            if dx.abs() + dy.abs() < size {
                                // diamond shape
                                let px = screen_x + dx;
                                let py = half_h as i32 + dy;
                                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                                    gfx.set_pixel_rgb(px as u32, py as u32, 30, 144, 255);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render remote players (green diamond)
        for rp in &self.remote_players {
            let sprite_pos = rp.body.position.sub(&self.player_body.position);
            let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
            let transform_x = inv_det * (self.dir.y * sprite_pos.x - self.dir.x * sprite_pos.y);
            let transform_y =
                inv_det * (-self.plane.y * sprite_pos.x + self.plane.x * sprite_pos.y);
            if transform_y > 0.1 && transform_y < 25.0 {
                let screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
                let size = ((14.0 / transform_y).abs() as i32).clamp(2, 16);
                if screen_x >= 0 && screen_x < w as i32 {
                    for dy in -size..=size {
                        for dx in -size..=size {
                            if dx.abs() + dy.abs() < size {
                                let px = screen_x + dx;
                                let py = half_h as i32 + dy;
                                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                                    gfx.set_pixel_rgb(px as u32, py as u32, 0, 220, 80);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render particles
        for particle in &self.particles {
            let sprite_pos = particle.position.sub(&self.player_body.position);
            let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
            let transform_x = inv_det * (self.dir.y * sprite_pos.x - self.dir.x * sprite_pos.y);
            let transform_y =
                inv_det * (-self.plane.y * sprite_pos.x + self.plane.x * sprite_pos.y);

            if transform_y > 0.1 && transform_y < 20.0 {
                let screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
                let size = ((8.0 / transform_y).abs() as i32).clamp(1, 10);

                if screen_x >= 0 && screen_x < w as i32 {
                    let zbuf_idx = screen_x as usize;
                    if zbuf_idx < z_buffer.len() && transform_y < z_buffer[zbuf_idx] {
                        let alpha = (particle.lifetime / particle.max_lifetime) as f32;
                        let r = (particle.color.0 as f32 * alpha) as u8;
                        let g = (particle.color.1 as f32 * alpha) as u8;
                        let b = (particle.color.2 as f32 * alpha) as u8;

                        for dy in -size..=size {
                            for dx in -size..=size {
                                let px = screen_x + dx;
                                let py = half_h as i32 + dy;
                                if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                                    gfx.set_pixel_rgb(px as u32, py as u32, r, g, b);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render projectiles
        for proj in &self.projectiles {
            let sprite_pos = proj.body.position.sub(&self.player_body.position);
            let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
            let transform_x = inv_det * (self.dir.y * sprite_pos.x - self.dir.x * sprite_pos.y);
            let transform_y =
                inv_det * (-self.plane.y * sprite_pos.x + self.plane.x * sprite_pos.y);

            if transform_y > 0.1 && transform_y < 20.0 {
                let screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
                let size = ((12.0 / transform_y).abs() as i32).max(2);

                if screen_x >= 0 && screen_x < w as i32 {
                    let zbuf_idx = screen_x as usize;
                    if zbuf_idx < z_buffer.len() && transform_y < z_buffer[zbuf_idx] {
                        for dy in -size..=size {
                            for dx in -size..=size {
                                if dx * dx + dy * dy <= size * size {
                                    let px = screen_x + dx;
                                    let py = half_h as i32 + dy;
                                    if px >= 0 && px < w as i32 && py >= 0 && py < h as i32 {
                                        gfx.set_pixel_rgb(px as u32, py as u32, 255, 255, 0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render monsters (sprite rendering with z-buffer)
        let mut sprites: Vec<(f64, &Monster)> = self
            .monsters
            .iter()
            .filter(|m| m.state != MonsterState::Dead)
            .filter_map(|m| {
                let sprite_pos = m.body.position.sub(&self.player_body.position);
                let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
                let transform_y =
                    inv_det * (-self.plane.y * sprite_pos.x + self.plane.x * sprite_pos.y);
                if transform_y > 0.1 {
                    Some((transform_y, m))
                } else {
                    None
                }
            })
            .collect();

        sprites.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (transform_y, monster) in sprites {
            let sprite_pos = monster.body.position.sub(&self.player_body.position);
            let inv_det = 1.0 / (self.plane.x * self.dir.y - self.dir.x * self.plane.y);
            let transform_x = inv_det * (self.dir.y * sprite_pos.x - self.dir.x * sprite_pos.y);

            let sprite_screen_x = ((w as f64 / 2.0) * (1.0 + transform_x / transform_y)) as i32;
            let sprite_height = ((h as f64 / transform_y).abs() as i32).min(h as i32 * 2);
            let sprite_width = sprite_height;

            if sprite_width <= 0 {
                continue;
            }

            let draw_start_y = ((half_h as i32 - sprite_height / 2).max(0)) as u32;
            let draw_end_y = ((half_h as i32 + sprite_height / 2).min(h as i32 - 1)) as u32;
            let draw_start_x = ((sprite_screen_x - sprite_width / 2).max(0)) as u32;
            let draw_end_x = ((sprite_screen_x + sprite_width / 2).min(w as i32 - 1)) as u32;

            if draw_end_y <= draw_start_y || draw_end_x <= draw_start_x {
                continue;
            }

            // Textured monster billboard
            let tex_index = if monster.sprite_type == 0 { 0 } else { 1 };
            let day_light = self.get_ambient_light();
            for stripe in draw_start_x..=draw_end_x {
                if stripe >= w {
                    break;
                }

                let zbuf_idx = stripe as usize;
                if zbuf_idx < z_buffer.len() && transform_y < z_buffer[zbuf_idx] {
                    for y in draw_start_y..=draw_end_y {
                        let tex_x = (stripe as i32 - (sprite_screen_x - sprite_width / 2)) as f64
                            / sprite_width as f64;
                        let tex_y = (y as i32 - (half_h as i32 - sprite_height / 2)) as f64
                            / sprite_height as f64;
                        if (0.0..=1.0).contains(&tex_x) && (0.0..=1.0).contains(&tex_y) {
                            let sx =
                                (tex_x * (MONSTER_TEX_W as f64)) as usize & (MONSTER_TEX_W - 1);
                            let sy =
                                (tex_y * (MONSTER_TEX_H as f64)) as usize & (MONSTER_TEX_H - 1);
                            let base = (sy * MONSTER_TEX_W + sx) * 3;
                            unsafe {
                                let mut r = MONSTER_TEXTURES[tex_index][base] as f32;
                                let mut g = MONSTER_TEXTURES[tex_index][base + 1] as f32;
                                let mut b = MONSTER_TEXTURES[tex_index][base + 2] as f32;
                                // Apply day light
                                r *= day_light;
                                g *= day_light;
                                b *= day_light;
                                gfx.set_pixel_rgb(stripe, y, r as u8, g as u8, b as u8);
                            }
                        }
                    }
                }
            }

            // Health bar above monster
            if draw_start_y > 10 {
                let bar_width = 30u32;
                let bar_y = draw_start_y - 5;
                let bar_x = (sprite_screen_x - bar_width as i32 / 2).max(0) as u32;
                let health_pct = (monster.health as f32 / monster.max_health as f32).max(0.0);
                let filled_width = (bar_width as f32 * health_pct) as u32;

                for x in 0..bar_width {
                    let bx = bar_x + x;
                    if bx < w {
                        let zbuf_idx = bx as usize;
                        if zbuf_idx < z_buffer.len() && transform_y < z_buffer[zbuf_idx] {
                            if x < filled_width {
                                gfx.set_pixel_rgb(bx, bar_y, 0, 255, 0);
                            } else {
                                gfx.set_pixel_rgb(bx, bar_y, 50, 50, 50);
                            }
                        }
                    }
                }
            }
        }

        // Draw HUD
        self.draw_hud(gfx);
        let _ = gfx.present();
    }

    fn get_sky_color(&self) -> (u8, u8, u8) {
        // 0.0 = midnight, 0.25 = dawn, 0.5 = noon, 0.75 = dusk, 1.0 = midnight
        if self.time_of_day < 0.25 {
            // Night to dawn
            let t = self.time_of_day * 4.0;
            let r = (20.0 + t * 80.0) as u8;
            let g = (30.0 + t * 120.0) as u8;
            let b = (60.0 + t * 160.0) as u8;
            (r, g, b)
        } else if self.time_of_day < 0.5 {
            // Dawn to noon
            let t = (self.time_of_day - 0.25) * 4.0;
            let r = (100.0 + t * 35.0) as u8;
            let g = (150.0 + t * 50.0) as u8;
            let b = 220;
            (r, g, b)
        } else if self.time_of_day < 0.75 {
            // Noon to dusk
            let t = (self.time_of_day - 0.5) * 4.0;
            let r = (135.0 - t * 35.0) as u8;
            let g = (200.0 - t * 80.0) as u8;
            let b = (220.0 - t * 80.0) as u8;
            (r, g, b)
        } else {
            // Dusk to night
            let t = (self.time_of_day - 0.75) * 4.0;
            let r = (100.0 - t * 80.0) as u8;
            let g = (120.0 - t * 90.0) as u8;
            let b = (140.0 - t * 80.0) as u8;
            (r, g, b)
        }
    }

    fn get_ambient_light(&self) -> f32 {
        // Full brightness during day, dimmer at night
        if self.time_of_day < 0.25 {
            (0.3 + self.time_of_day * 2.8) as f32
        } else if self.time_of_day < 0.75 {
            1.0
        } else {
            (1.0 - (self.time_of_day - 0.75) * 2.8) as f32
        }
    }

    fn draw_hud(&self, gfx: &mut Renderer) {
        let w = gfx.width();
        let h = gfx.height();

        // Health bar
        let bar_width = 200u32;
        let bar_height = 20u32;
        let bar_x = 20u32;
        let bar_y = h - 50;

        let health_pct = (self.health as f32 / self.max_health as f32).max(0.0);
        let filled = (bar_width as f32 * health_pct) as u32;

        // Background
        gfx.fill_rect(bar_x, bar_y, bar_width, bar_height, 30, 30, 30);
        // Health fill
        if filled > 0 {
            let r = if health_pct > 0.5 { 0 } else { 255 };
            let g = if health_pct > 0.5 {
                200
            } else {
                (health_pct * 400.0) as u8
            };
            gfx.fill_rect(bar_x, bar_y, filled, bar_height, r, g, 0);
        }

        // Ammo / weapon indicator
        let ammo_x = w - 140;
        let ammo_y = h - 55;
        if self.current_weapon == 0 {
            // infinite pistol
            self.draw_text(gfx, "AMMO", ammo_x, ammo_y, (200, 200, 200));
            self.draw_text(gfx, "INF", ammo_x, ammo_y + 16, (255, 255, 0));
        } else {
            self.draw_text(gfx, "AMMO", ammo_x, ammo_y, (200, 200, 200));
            self.draw_number(gfx, self.ammo, ammo_x, ammo_y + 16);
            if self.ammo < 10 {
                // low ammo flash border
                gfx.draw_rect(ammo_x - 4, ammo_y + 12, 70, 22, 255, 0, 0);
            }
        }

        // Score
        let score_x = w / 2 - 60;
        let score_y = 20;
        self.draw_number(gfx, self.score as i32, score_x, score_y);

        // Difficulty indicator text
        let diff_x = 20;
        let diff_y = 20;
        let (diff_str, diff_color) = match self.difficulty {
            Difficulty::Easy => ("EASY", (0, 255, 0)),
            Difficulty::Normal => ("NORMAL", (255, 255, 0)),
            Difficulty::Hard => ("HARD", (255, 0, 0)),
        };
        self.draw_text(gfx, diff_str, diff_x, diff_y, diff_color);

        // Remote player count (multiplayer indicator)
        if !self.remote_players.is_empty() {
            let mp_str = format!("MP:{}", self.remote_players.len());
            self.draw_text(gfx, &mp_str, diff_x, diff_y + 18, (0, 180, 255));
        }

        // Crosshair
        let cx = w / 2;
        let cy = h / 2;
        gfx.draw_hline(cx - 10, cx + 10, cy, 255, 255, 255);
        gfx.draw_vline(cx, cy - 10, cy + 10, 255, 255, 255);
    }

    fn draw_number(&self, gfx: &mut Renderer, num: i32, x: u32, y: u32) {
        // 5x7 pixel font for digits 0-9 scaled by scale factor
        const SCALE: u32 = 2; // Each font pixel becomes SCALE x SCALE block
        const FONT_W: u32 = 5;
        const FONT_H: u32 = 7;
        static DIGITS: [&str; 10] = [
            // Each string is 5*7=35 chars of '1' (on) or '0' (off), row-major
            // 0
            "11111\
             10001\
             10011\
             10101\
             11001\
             10001\
             11111",
            // 1
            "00100\
             01100\
             00100\
             00100\
             00100\
             00100\
             01110",
            // 2
            "11111\
             00001\
             00001\
             11111\
             10000\
             10000\
             11111",
            // 3
            "11111\
             00001\
             00001\
             11111\
             00001\
             00001\
             11111",
            // 4
            "10001\
             10001\
             10001\
             11111\
             00001\
             00001\
             00001",
            // 5
            "11111\
             10000\
             10000\
             11111\
             00001\
             00001\
             11111",
            // 6
            "11111\
             10000\
             10000\
             11111\
             10001\
             10001\
             11111",
            // 7
            "11111\
             00001\
             00001\
             00010\
             00010\
             00100\
             00100",
            // 8
            "11111\
             10001\
             10001\
             11111\
             10001\
             10001\
             11111",
            // 9
            "11111\
             10001\
             10001\
             11111\
             00001\
             00001\
             11111",
        ];

        let digits = num.to_string();
        let mut offset = 0;
        for ch in digits.chars() {
            if let Some(d) = ch.to_digit(10) {
                let pattern = DIGITS[d as usize].replace('\n', "");
                for fy in 0..FONT_H {
                    for fx in 0..FONT_W {
                        let idx = (fy * FONT_W + fx) as usize;
                        if pattern.as_bytes()[idx] == b'1' {
                            // draw scaled block
                            for sx in 0..SCALE {
                                for sy in 0..SCALE {
                                    let px = x + offset + fx * SCALE + sx;
                                    let py = y + fy * SCALE + sy;
                                    if px < gfx.width() && py < gfx.height() {
                                        gfx.set_pixel_rgb(px, py, 255, 255, 0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            offset += (FONT_W + 1) * SCALE; // 1 pixel spacing
        }
    }

    fn draw_text(&self, gfx: &mut Renderer, text: &str, x: u32, y: u32, color: (u8, u8, u8)) {
        // 5x7 uppercase font (subset) scaled by 2
        const SCALE: u32 = 2;
        const W: u32 = 5;
        const H: u32 = 7;
        fn pattern(ch: char) -> &'static str {
            match ch {
                'A' => "01110\n10001\n10001\n11111\n10001\n10001\n10001",
                'D' => "11110\n10001\n10001\n10001\n10001\n10001\n11110",
                'E' => "11111\n10000\n10000\n11111\n10000\n10000\n11111",
                'F' => "11111\n10000\n10000\n11111\n10000\n10000\n10000",
                'H' => "10001\n10001\n10001\n11111\n10001\n10001\n10001",
                'I' => "11111\n00100\n00100\n00100\n00100\n00100\n11111",
                'L' => "10000\n10000\n10000\n10000\n10000\n10000\n11111",
                'M' => "10001\n11011\n10101\n10101\n10001\n10001\n10001",
                'N' => "10001\n11001\n10101\n10011\n10001\n10001\n10001",
                'O' => "01110\n10001\n10001\n10001\n10001\n10001\n01110",
                'P' => "11110\n10001\n10001\n11110\n10000\n10000\n10000",
                'R' => "11110\n10001\n10001\n11110\n10100\n10010\n10001",
                'S' => "01111\n10000\n10000\n01110\n00001\n00001\n11110",
                'Y' => "10001\n10001\n01010\n00100\n00100\n00100\n00100",
                '0' => "01110\n10001\n10011\n10101\n11001\n10001\n01110",
                '1' => "00100\n01100\n00100\n00100\n00100\n00100\n01110",
                '2' => "11111\n00001\n00001\n11111\n10000\n10000\n11111",
                '3' => "11111\n00001\n00001\n11111\n00001\n00001\n11111",
                ':' => "00000\n00100\n00100\n00000\n00100\n00100\n00000",
                _ => "00000\n00000\n00000\n00000\n00000\n00000\n00000",
            }
        }
        let mut offset = 0;
        for ch in text.chars() {
            let pat = pattern(ch).replace('\n', "");
            for fy in 0..H {
                for fx in 0..W {
                    let idx = (fy * W + fx) as usize;
                    if pat.as_bytes()[idx] == b'1' {
                        for sx in 0..SCALE {
                            for sy in 0..SCALE {
                                let px = x + offset + fx * SCALE + sx;
                                let py = y + fy * SCALE + sy;
                                if px < gfx.width() && py < gfx.height() {
                                    gfx.set_pixel_rgb(px, py, color.0, color.1, color.2);
                                }
                            }
                        }
                    }
                }
            }
            offset += (W + 1) * SCALE;
        }
    }
}

#[derive(Clone)]
struct RemotePlayer {
    id: String,
    body: Body,
}

// Multiplayer WASM bindings
#[wasm_bindgen]
pub fn doom_add_remote_player(id: &str, x: f64, y: f64) {
    GAME.with(|gm| {
        if let Some(ref mut game) = *gm.borrow_mut() {
            if game.remote_players.iter().any(|p| p.id == id) {
                return;
            }
            let mut body = Body::new(x, y, 0.3);
            body.friction = 0.3;
            game.remote_players.push(RemotePlayer {
                id: id.to_string(),
                body,
            });
        }
    });
}

#[wasm_bindgen]
pub fn doom_update_remote_player(id: &str, x: f64, y: f64) {
    GAME.with(|gm| {
        if let Some(ref mut game) = *gm.borrow_mut() {
            if let Some(p) = game.remote_players.iter_mut().find(|p| p.id == id) {
                p.body.position.x = x;
                p.body.position.y = y;
            }
        }
    });
}

#[wasm_bindgen]
pub fn doom_remove_remote_player(id: &str) {
    GAME.with(|gm| {
        if let Some(ref mut game) = *gm.borrow_mut() {
            game.remote_players.retain(|p| p.id != id);
        }
    });
}

#[wasm_bindgen]
pub fn doom_enable_procedural() {
    GAME.with(|gm| {
        if let Some(ref mut game) = *gm.borrow_mut() {
            game.enable_procedural();
        }
    });
}

#[wasm_bindgen]
pub fn doom_restore_original_map() {
    restore_original_map();
}

#[wasm_bindgen]
pub fn doom_get_player_position() -> js_sys::Array {
    let arr = js_sys::Array::new();
    GAME.with(|gm| {
        if let Some(ref game) = *gm.borrow() {
            arr.push(&JsValue::from_f64(game.player_body.position.x));
            arr.push(&JsValue::from_f64(game.player_body.position.y));
        }
    });
    arr
}

thread_local! {
    static MP_ID: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static MP_PC: std::cell::RefCell<Option<RtcPeerConnection>> = const { std::cell::RefCell::new(None) };
    static MP_CHAN: std::cell::RefCell<Option<RtcDataChannel>> = const { std::cell::RefCell::new(None) };
    static MP_INTERVAL: std::cell::RefCell<Option<i32>> = const { std::cell::RefCell::new(None) };
    static MP_HOSTING: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
}

fn local_player_id() -> String {
    MP_ID.with(|id| {
        if id.borrow().is_empty() {
            let rand = js_sys::Math::random();
            let s = format!("{:08x}", (rand * 0xffff_ffffu32 as f64) as u32);
            *id.borrow_mut() = s;
        }
        id.borrow().clone()
    })
}

fn broadcast_position() {
    let pid = local_player_id();
    let (x, y) = GAME.with(|gm| {
        if let Some(ref g) = *gm.borrow() {
            (g.player_body.position.x, g.player_body.position.y)
        } else {
            (0.0, 0.0)
        }
    });
    let msg = serde_json::json!({"t":"pos","id":pid,"x":x,"y":y}).to_string();
    MP_CHAN.with(|ch| {
        if let Some(ref dc) = *ch.borrow() {
            let _ = dc.send_with_str(&msg);
        }
    });
}

fn handle_incoming(data: &str) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
        let t = val.get("t").and_then(|v| v.as_str()).unwrap_or("");
        let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id == local_player_id() {
            return;
        } // ignore own
        match t {
            "join" => {
                let x = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                GAME.with(|gm| {
                    if let Some(ref mut g) = *gm.borrow_mut() {
                        if !g.remote_players.iter().any(|p| p.id == id) {
                            let mut body = Body::new(x, y, 0.3);
                            body.friction = 0.3;
                            g.remote_players.push(RemotePlayer {
                                id: id.to_string(),
                                body,
                            });
                        }
                    }
                });
            }
            "leave" => {
                GAME.with(|gm| {
                    if let Some(ref mut g) = *gm.borrow_mut() {
                        g.remote_players.retain(|p| p.id != id);
                    }
                });
            }
            "pos" => {
                let x = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                GAME.with(|gm| {
                    if let Some(ref mut g) = *gm.borrow_mut() {
                        if let Some(p) = g.remote_players.iter_mut().find(|p| p.id == id) {
                            p.body.position.x = x;
                            p.body.position.y = y;
                        }
                    }
                });
            }
            _ => {}
        }
    }
}

fn setup_interval() {
    MP_INTERVAL.with(|iv| {
        if iv.borrow().is_some() {
            return;
        }
        let cb = Closure::<dyn FnMut()>::wrap(Box::new(broadcast_position));
        let handle = window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                200,
            )
            .unwrap();
        cb.forget();
        *iv.borrow_mut() = Some(handle);
    });
}

#[wasm_bindgen]
pub fn mp_id() -> String {
    local_player_id()
}

#[wasm_bindgen]
pub async fn mp_host() -> Result<String, JsValue> {
    MP_HOSTING.with(|h| *h.borrow_mut() = true);
    let pc = RtcPeerConnection::new()?;
    let channel = pc.create_data_channel("doom");
    MP_CHAN.with(|c| *c.borrow_mut() = Some(channel.clone()));
    let onmsg = Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(|e: MessageEvent| {
        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            handle_incoming(&String::from(txt));
        }
    }));
    channel.set_onmessage(Some(onmsg.as_ref().unchecked_ref()));
    onmsg.forget();
    MP_PC.with(|p| *p.borrow_mut() = Some(pc.clone()));
    let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer()).await?;
    let offer_sdp_initial = Reflect::get(&offer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap_or_default();
    let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    desc.set_sdp(&offer_sdp_initial);
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc)).await?;
    // Return initial SDP (without waiting for full ICE gathering to avoid unsupported APIs)
    let code = format!("{}:{}", encode_sdp(&offer_sdp_initial), local_player_id());
    Ok(code)
}

#[wasm_bindgen]
pub async fn mp_join(code: &str) -> Result<String, JsValue> {
    let mut parts = code.splitn(2, ':');
    let offer_enc = parts.next().unwrap_or("");
    let host_id = parts.next().unwrap_or("");
    let offer_sdp = decode_sdp(offer_enc);
    console::log_1(&JsValue::from_str(&format!(
        "[MP_JOIN] Decoded SDP:\n{}",
        offer_sdp
    )));
    let pc = RtcPeerConnection::new()?;
    MP_PC.with(|p| *p.borrow_mut() = Some(pc.clone()));
    let ondc = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(|e: web_sys::Event| {
        if let Some(dc) = e.target().and_then(|t| t.dyn_into::<RtcDataChannel>().ok()) {
            MP_CHAN.with(|c| *c.borrow_mut() = Some(dc.clone()));
            let onmsg = Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(|ev: MessageEvent| {
                if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                    handle_incoming(&String::from(txt));
                }
            }));
            dc.set_onmessage(Some(onmsg.as_ref().unchecked_ref()));
            onmsg.forget();
            // Send join message when open
            let onopen = Closure::<dyn FnMut()>::wrap(Box::new(|| {
                let pid = local_player_id();
                let msg = serde_json::json!({"t":"join","id":pid}).to_string();
                MP_CHAN.with(|c| {
                    if let Some(ref ch) = *c.borrow() {
                        let _ = ch.send_with_str(&msg);
                    }
                });
            }));
            dc.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();
        }
    }));
    pc.set_ondatachannel(Some(ondc.as_ref().unchecked_ref()));
    ondc.forget();
    let offer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_desc.set_sdp(&offer_sdp);
    wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_desc)).await?;
    let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer()).await?;
    let answer_sdp_initial = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap_or_default();
    let answer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_desc.set_sdp(&answer_sdp_initial);
    wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_desc)).await?;
    // Return initial SDP (without waiting for full ICE gathering)
    let answer_code = format!("{}:{}", encode_sdp(&answer_sdp_initial), host_id);
    Ok(answer_code)
}

#[wasm_bindgen]
pub async fn mp_finalize(answer_code: &str) -> Result<(), JsValue> {
    let mut parts = answer_code.splitn(2, ':');
    let answer_enc = parts.next().unwrap_or("");
    let _host_id = parts.next().unwrap_or("");
    let answer_sdp = decode_sdp(answer_enc);
    console::log_1(&JsValue::from_str(&format!(
        "[MP_FINALIZE] Decoded SDP:\n{}",
        answer_sdp
    )));
    let answer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer_desc.set_sdp(&answer_sdp);
    let pc_opt = MP_PC.with(|p| p.borrow().clone());
    if let Some(pc) = pc_opt {
        wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_desc)).await?;
        // After channel open send join
        MP_CHAN.with(|c| {
            if let Some(ref ch) = *c.borrow() {
                let pid = local_player_id();
                let msg = serde_json::json!({"t":"join","id":pid}).to_string();
                let _ = ch.send_with_str(&msg);
            }
        });
        setup_interval();
    }
    Ok(())
}

#[wasm_bindgen]
pub fn mp_disconnect() {
    MP_INTERVAL.with(|iv| {
        if let Some(id) = *iv.borrow() {
            window().unwrap().clear_interval_with_handle(id);
        }
        *iv.borrow_mut() = None;
    });
    MP_CHAN.with(|c| *c.borrow_mut() = None);
    MP_PC.with(|p| {
        if let Some(pc) = p.borrow().clone() {
            pc.close();
        }
    });
}

impl Monster {
    fn new(x: f64, y: f64, sprite_type: u8, difficulty: Difficulty) -> Self {
        let max_health = match (sprite_type, difficulty) {
            (0, Difficulty::Easy) => 40,
            (0, Difficulty::Normal) => 60,
            (0, Difficulty::Hard) => 80,
            (1, Difficulty::Easy) => 60,
            (1, Difficulty::Normal) => 100,
            (1, Difficulty::Hard) => 150,
            _ => 60,
        };

        let mut body = Body::new(x, y, 0.3);
        body.mass = 2.0;
        body.friction = 0.2;

        Monster {
            body,
            health: max_health,
            max_health,
            sprite_type,
            state: MonsterState::Idle,
            attack_cooldown: 0.0,
        }
    }
}

// Simple sound synthesis using Web Audio API
fn play_sound(frequency: f64, duration: f64) {
    AUDIO_CTX.with(|ctx_cell| {
        if ctx_cell.borrow().is_none() {
            if let Ok(audio_ctx) = AudioContext::new() {
                *ctx_cell.borrow_mut() = Some(audio_ctx);
            }
        }

        if let Some(ctx) = ctx_cell.borrow().as_ref() {
            if let Ok(oscillator) = ctx.create_oscillator() {
                if let Ok(gain) = ctx.create_gain() {
                    oscillator.set_type(OscillatorType::Square);
                    oscillator.frequency().set_value(frequency as f32);
                    oscillator.connect_with_audio_node(&gain).ok();
                    gain.connect_with_audio_node(&ctx.destination()).ok();
                    gain.gain().set_value(0.1);

                    let now = ctx.current_time();
                    oscillator.start_with_when(now).ok();
                    oscillator.stop_with_when(now + duration).ok();
                }
            }
        }
    });
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
        if rcb.borrow().is_some() {
            return;
        }
        let cb = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(|_e: web_sys::Event| {
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

        let mut last_time = js_sys::Date::now();

        let closure = Closure::wrap(Box::new(move |_ts: f64| {
            let stopping = STOPPING.with(|s| s.get());
            if stopping {
                return;
            }

            let still_running = GAME.with(|g| g.borrow().is_some());
            if !still_running {
                return;
            }

            // Calculate delta time
            let now = js_sys::Date::now();
            let dt = ((now - last_time) / 1000.0).min(0.05); // Cap at 50ms
            last_time = now;

            let should_stop = GAME.with(|g| {
                if let Some(ref mut game) = *g.borrow_mut() {
                    game.update(dt)
                } else {
                    false
                }
            });

            if should_stop {
                stop_doom();
                return;
            }

            let can_render =
                GAME.with(|g| g.borrow().is_some()) && GFX.with(|gfx| gfx.borrow().is_some());
            if can_render {
                GAME.with(|g| {
                    if let Some(ref game) = *g.borrow() {
                        GFX.with(|gfx| {
                            if let Some(ref mut graphics) = *gfx.borrow_mut() {
                                game.render(graphics);
                            }
                        });
                    }
                });
            }

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
    start_doom_with_difficulty(1); // Default to Normal
}

#[wasm_bindgen]
pub fn start_doom_with_difficulty(diff: u8) {
    let difficulty = match diff {
        0 => Difficulty::Easy,
        2 => Difficulty::Hard,
        _ => Difficulty::Normal,
    };

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
        *gm.borrow_mut() = Some(DoomGame::new(difficulty));
    });

    update_canvas_size();
    install_resize_listener();
    crate::idle::set_game_active(true);
    crate::idle::set_screensaver_active(false);
    start_loop();
}

#[wasm_bindgen]
pub fn stop_doom() {
    STOPPING.with(|s| s.set(true));
    LOOP.with(|l| {
        *l.borrow_mut() = None;
    });
    uninstall_resize_listener();

    GAME.with(|gm| {
        *gm.borrow_mut() = None;
    });
    GFX.with(|gfx| {
        *gfx.borrow_mut() = None;
    });

    KEYS.with(|k| {
        let mut keys = k.borrow_mut();
        for i in 0..keys.len() {
            keys[i] = false;
        }
    });
    MOUSE_DELTA_X.with(|md| md.set(0.0));
    MOUSE_CLICKED.with(|mc| mc.set(false));

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

#[wasm_bindgen]
pub fn memory_usage() -> String {
    let mem = wasm_bindgen::memory();
    let buf_val = js_sys::Reflect::get(&mem, &JsValue::from_str("buffer")).ok();
    let bytes = buf_val
        .map(|bv| {
            let ua = js_sys::Uint8Array::new(&bv);
            ua.length() as u64
        })
        .unwrap_or(0);
    let (monsters, projectiles, particles) = GAME.with(|g| {
        if let Some(ref game) = *g.borrow() {
            (
                game.monsters.len(),
                game.projectiles.len(),
                game.particles.len(),
            )
        } else {
            (0, 0, 0)
        }
    });
    format!(
        "wasm_bytes={} monsters={} projectiles={} particles={}",
        bytes, monsters, projectiles, particles
    )
}
