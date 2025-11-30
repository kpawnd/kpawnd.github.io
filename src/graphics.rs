use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    #[inline(always)]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    #[inline(always)]
    pub const fn rgba_u32(&self) -> u32 {
        (self.a as u32) << 24 | (self.b as u32) << 16 | (self.g as u32) << 8 | (self.r as u32)
    }

    pub const BLACK: Color = Color::new(0, 0, 0, 255);
    pub const WHITE: Color = Color::new(255, 255, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0, 255);
    pub const GREEN: Color = Color::new(0, 255, 0, 255);
    pub const BLUE: Color = Color::new(0, 0, 255, 255);
    pub const YELLOW: Color = Color::new(255, 255, 0, 255);
    pub const CYAN: Color = Color::new(0, 255, 255, 255);
    pub const MAGENTA: Color = Color::new(255, 0, 255, 255);
}

/// High-performance frame buffer with batch operations
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    // Cached values for fast access
    pub stride: usize,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        let pixels = vec![0; size];
        FrameBuffer {
            width,
            height,
            pixels,
            stride: (width * 4) as usize,
        }
    }

    /// Ultra-fast clear using memset-like pattern
    #[inline]
    pub fn clear(&mut self, color: &Color) {
        // Create a 4-byte pattern that can be repeated
        let pattern = [color.r, color.g, color.b, color.a];

        // Clear in chunks for cache efficiency
        let len = self.pixels.len();
        let mut i = 0;

        // Unroll loop 8x for better performance
        while i + 32 <= len {
            unsafe {
                let ptr = self.pixels.as_mut_ptr().add(i);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr, 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(4), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(8), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(12), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(16), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(20), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(24), 4);
                std::ptr::copy_nonoverlapping(pattern.as_ptr(), ptr.add(28), 4);
            }
            i += 32;
        }

        // Handle remainder
        while i + 4 <= len {
            self.pixels[i] = color.r;
            self.pixels[i + 1] = color.g;
            self.pixels[i + 2] = color.b;
            self.pixels[i + 3] = color.a;
            i += 4;
        }
    }

    /// Fast clear to black (optimized memset to 0)
    #[inline]
    pub fn clear_black(&mut self) {
        self.pixels.fill(0);
        // Set alpha channel to 255
        let len = self.pixels.len();
        let mut i = 3;
        while i < len {
            self.pixels[i] = 255;
            i += 4;
        }
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: &Color) {
        if x < self.width && y < self.height {
            let idx = (y as usize * self.stride) + (x as usize * 4);
            unsafe {
                *self.pixels.get_unchecked_mut(idx) = color.r;
                *self.pixels.get_unchecked_mut(idx + 1) = color.g;
                *self.pixels.get_unchecked_mut(idx + 2) = color.b;
                *self.pixels.get_unchecked_mut(idx + 3) = color.a;
            }
        }
    }

    /// Set pixel with raw RGB values (no Color struct allocation)
    #[inline(always)]
    pub fn set_pixel_rgb(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x < self.width && y < self.height {
            let idx = (y as usize * self.stride) + (x as usize * 4);
            unsafe {
                *self.pixels.get_unchecked_mut(idx) = r;
                *self.pixels.get_unchecked_mut(idx + 1) = g;
                *self.pixels.get_unchecked_mut(idx + 2) = b;
                *self.pixels.get_unchecked_mut(idx + 3) = 255;
            }
        }
    }

    /// Ultra-fast unchecked pixel set (caller must ensure bounds)
    ///
    /// # Safety
    ///
    /// Caller must ensure that x < width and y < height, otherwise this will
    /// cause undefined behavior by accessing out-of-bounds memory.
    #[inline(always)]
    pub unsafe fn set_pixel_unchecked(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let idx = (y as usize * self.stride) + (x as usize * 4);
        *self.pixels.get_unchecked_mut(idx) = r;
        *self.pixels.get_unchecked_mut(idx + 1) = g;
        *self.pixels.get_unchecked_mut(idx + 2) = b;
        *self.pixels.get_unchecked_mut(idx + 3) = 255;
    }

    /// Draw a vertical line (common in raycasting) - highly optimized
    #[inline]
    pub fn draw_vline(&mut self, x: u32, y_start: u32, y_end: u32, r: u8, g: u8, b: u8) {
        if x >= self.width {
            return;
        }
        let y0 = y_start.min(self.height - 1);
        let y1 = y_end.min(self.height - 1);

        let mut idx = (y0 as usize * self.stride) + (x as usize * 4);
        for _ in y0..=y1 {
            unsafe {
                *self.pixels.get_unchecked_mut(idx) = r;
                *self.pixels.get_unchecked_mut(idx + 1) = g;
                *self.pixels.get_unchecked_mut(idx + 2) = b;
                *self.pixels.get_unchecked_mut(idx + 3) = 255;
            }
            idx += self.stride;
        }
    }

    /// Draw a vertical line with depth-based shading
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn draw_vline_shaded(
        &mut self,
        x: u32,
        y_start: u32,
        y_end: u32,
        r: u8,
        g: u8,
        b: u8,
        shade: f32,
    ) {
        if x >= self.width {
            return;
        }
        let y0 = y_start.min(self.height - 1);
        let y1 = y_end.min(self.height - 1);

        let sr = (r as f32 * shade) as u8;
        let sg = (g as f32 * shade) as u8;
        let sb = (b as f32 * shade) as u8;

        let mut idx = (y0 as usize * self.stride) + (x as usize * 4);
        for _ in y0..=y1 {
            unsafe {
                *self.pixels.get_unchecked_mut(idx) = sr;
                *self.pixels.get_unchecked_mut(idx + 1) = sg;
                *self.pixels.get_unchecked_mut(idx + 2) = sb;
                *self.pixels.get_unchecked_mut(idx + 3) = 255;
            }
            idx += self.stride;
        }
    }

    /// Draw horizontal line (optimized with memset-like approach)
    #[inline]
    pub fn draw_hline(&mut self, x_start: u32, x_end: u32, y: u32, r: u8, g: u8, b: u8) {
        if y >= self.height {
            return;
        }
        let x0 = x_start.min(self.width - 1);
        let x1 = x_end.min(self.width - 1);

        let row_start = (y as usize * self.stride) + (x0 as usize * 4);
        for x in x0..=x1 {
            let idx = row_start + ((x - x0) as usize * 4);
            unsafe {
                *self.pixels.get_unchecked_mut(idx) = r;
                *self.pixels.get_unchecked_mut(idx + 1) = g;
                *self.pixels.get_unchecked_mut(idx + 2) = b;
                *self.pixels.get_unchecked_mut(idx + 3) = 255;
            }
        }
    }

    /// Fill a horizontal span (for floor/ceiling casting)
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn fill_hspan_gradient(
        &mut self,
        x_start: u32,
        x_end: u32,
        y: u32,
        r1: u8,
        g1: u8,
        b1: u8,
        r2: u8,
        g2: u8,
        b2: u8,
    ) {
        if y >= self.height || x_start >= x_end {
            return;
        }
        let x0 = x_start.min(self.width - 1);
        let x1 = x_end.min(self.width - 1);
        let span = (x1 - x0) as f32;

        let row_start = (y as usize * self.stride) + (x0 as usize * 4);
        for x in 0..(x1 - x0) {
            let t = x as f32 / span;
            let idx = row_start + (x as usize * 4);
            unsafe {
                *self.pixels.get_unchecked_mut(idx) =
                    (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8;
                *self.pixels.get_unchecked_mut(idx + 1) =
                    (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8;
                *self.pixels.get_unchecked_mut(idx + 2) =
                    (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8;
                *self.pixels.get_unchecked_mut(idx + 3) = 255;
            }
        }
    }

    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: &Color) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);

        for dy in y..y_end {
            let row_start = dy as usize * self.stride;
            for dx in x..x_end {
                let idx = row_start + (dx as usize * 4);
                if idx + 3 < self.pixels.len() {
                    self.pixels[idx] = color.r;
                    self.pixels[idx + 1] = color.g;
                    self.pixels[idx + 2] = color.b;
                    self.pixels[idx + 3] = color.a;
                }
            }
        }
    }

    /// Optimized filled rectangle with raw colors
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);

        for dy in y..y_end {
            self.draw_hline(x, x_end - 1, dy, r, g, b);
        }
    }

    pub fn draw_circle(&mut self, cx: u32, cy: u32, radius: u32, color: &Color) {
        let r2 = (radius * radius) as i32;
        for dy in 0..=radius {
            for dx in 0..=radius {
                if (dx * dx + dy * dy) as i32 <= r2 {
                    if cx >= dx && cy >= dy {
                        self.set_pixel(cx - dx, cy - dy, color);
                    }
                    self.set_pixel(cx + dx, cy + dy, color);
                    if cx >= dx {
                        self.set_pixel(cx - dx, cy + dy, color);
                    }
                    if cy >= dy {
                        self.set_pixel(cx + dx, cy - dy, color);
                    }
                }
            }
        }
    }

    pub fn draw_line(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, color: &Color) {
        let dx = (x1 as i32 - x0 as i32).abs();
        let dy = -(y1 as i32 - y0 as i32).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x0 as i32;
        let mut y = y0 as i32;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as u32, y as u32, color);
            }
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
}

#[wasm_bindgen]
pub struct Graphics {
    #[allow(dead_code)]
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    buffer: FrameBuffer,
}

#[wasm_bindgen]
impl Graphics {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Result<Graphics, JsValue> {
        let document = web_sys::window()
            .ok_or("No window")?
            .document()
            .ok_or("No document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;

        canvas.set_width(width);
        canvas.set_height(height);

        let context = canvas
            .get_context("2d")?
            .ok_or("Failed to get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let buffer = FrameBuffer::new(width, height);

        Ok(Graphics {
            canvas,
            context,
            buffer,
        })
    }

    pub fn width(&self) -> u32 {
        self.buffer.width
    }

    pub fn height(&self) -> u32 {
        self.buffer.height
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        let color = Color::rgb(r, g, b);
        self.buffer.clear(&color);
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let color = Color::rgb(r, g, b);
        self.buffer.set_pixel(x, y, &color);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        let color = Color::rgb(r, g, b);
        self.buffer.draw_rect(x, y, w, h, &color);
    }

    pub fn draw_circle(&mut self, cx: u32, cy: u32, radius: u32, r: u8, g: u8, b: u8) {
        let color = Color::rgb(r, g, b);
        self.buffer.draw_circle(cx, cy, radius, &color);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_line(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, r: u8, g: u8, b: u8) {
        let color = Color::rgb(r, g, b);
        self.buffer.draw_line(x0, y0, x1, y1, &color);
    }

    pub fn present(&self) -> Result<(), JsValue> {
        let expected_size = (self.buffer.width * self.buffer.height * 4) as usize;
        if self.buffer.pixels.len() != expected_size {
            web_sys::console::error_1(
                &format!(
                    "Buffer size mismatch: expected {}, got {}. Dimensions: {}x{}",
                    expected_size,
                    self.buffer.pixels.len(),
                    self.buffer.width,
                    self.buffer.height
                )
                .into(),
            );
            return Err(JsValue::from_str("Buffer size mismatch"));
        }
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&self.buffer.pixels),
            self.buffer.width,
            self.buffer.height,
        )?;
        self.context.put_image_data(&image_data, 0.0, 0.0)?;
        Ok(())
    }

    // Buffer access methods for direct pixel manipulation (used by DOOM)
    pub fn set_pixel_unchecked(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        unsafe {
            self.buffer.set_pixel_unchecked(x, y, r, g, b);
        }
    }

    pub fn set_pixel_rgb(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        self.buffer.set_pixel_rgb(x, y, r, g, b);
    }

    pub fn draw_vline(&mut self, x: u32, y_start: u32, y_end: u32, r: u8, g: u8, b: u8) {
        self.buffer.draw_vline(x, y_start, y_end, r, g, b);
    }

    pub fn draw_hline(&mut self, x_start: u32, x_end: u32, y: u32, r: u8, g: u8, b: u8) {
        self.buffer.draw_hline(x_start, x_end, y, r, g, b);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        self.buffer.fill_rect(x, y, w, h, r, g, b);
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        if width == self.buffer.width && height == self.buffer.height {
            return Ok(());
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.buffer = FrameBuffer::new(width, height);
        Ok(())
    }
}

// Snake Game Implementation
#[wasm_bindgen]
pub struct SnakeGame {
    width: u32,
    height: u32,
    cell_size: u32,
    snake: Vec<(u32, u32)>,
    direction: Direction,
    food: (u32, u32),
    game_over: bool,
    score: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[wasm_bindgen]
impl SnakeGame {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32, cell_size: u32) -> Self {
        let grid_w = width / cell_size;
        let grid_h = height / cell_size;
        let snake = vec![(grid_w / 2, grid_h / 2)];
        let food = (grid_w / 4, grid_h / 4);

        SnakeGame {
            width,
            height,
            cell_size,
            snake,
            direction: Direction::Right,
            food,
            game_over: false,
            score: 0,
        }
    }

    pub fn update(&mut self) {
        if self.game_over {
            return;
        }

        let head = self.snake[0];
        let new_head = match self.direction {
            Direction::Up => (head.0, head.1.wrapping_sub(1)),
            Direction::Down => (head.0, head.1 + 1),
            Direction::Left => (head.0.wrapping_sub(1), head.1),
            Direction::Right => (head.0 + 1, head.1),
        };

        let grid_w = self.width / self.cell_size;
        let grid_h = self.height / self.cell_size;

        // Check collision with walls
        if new_head.0 >= grid_w || new_head.1 >= grid_h {
            self.game_over = true;
            return;
        }

        // Check collision with self
        if self.snake.contains(&new_head) {
            self.game_over = true;
            return;
        }

        self.snake.insert(0, new_head);

        // Check if food eaten
        if new_head == self.food {
            self.score += 10;
            self.spawn_food();
        } else {
            self.snake.pop();
        }
    }

    pub fn render(&self, gfx: &mut Graphics) {
        // Clear background
        gfx.clear(20, 20, 20);

        // Draw snake
        for (i, &(x, y)) in self.snake.iter().enumerate() {
            let color = if i == 0 {
                (0, 255, 0) // Head - bright green
            } else {
                (0, 200, 0) // Body - darker green
            };
            gfx.draw_rect(
                x * self.cell_size,
                y * self.cell_size,
                self.cell_size - 1,
                self.cell_size - 1,
                color.0,
                color.1,
                color.2,
            );
        }

        // Draw food
        gfx.draw_rect(
            self.food.0 * self.cell_size,
            self.food.1 * self.cell_size,
            self.cell_size - 1,
            self.cell_size - 1,
            255,
            0,
            0,
        );
    }

    pub fn set_direction(&mut self, dir: &str) {
        let new_dir = match dir {
            "up" => Direction::Up,
            "down" => Direction::Down,
            "left" => Direction::Left,
            "right" => Direction::Right,
            _ => return,
        };

        // Prevent 180 degree turns
        let opposite = matches!(
            (self.direction, new_dir),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        );

        if !opposite {
            self.direction = new_dir;
        }
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn reset(&mut self) {
        let grid_w = self.width / self.cell_size;
        let grid_h = self.height / self.cell_size;
        self.snake = vec![(grid_w / 2, grid_h / 2)];
        self.direction = Direction::Right;
        self.game_over = false;
        self.score = 0;
        self.spawn_food();
    }

    fn spawn_food(&mut self) {
        let grid_w = self.width / self.cell_size;
        let grid_h = self.height / self.cell_size;

        loop {
            let x = (js_sys::Math::random() * grid_w as f64) as u32;
            let y = (js_sys::Math::random() * grid_h as f64) as u32;
            let pos = (x, y);

            if !self.snake.contains(&pos) {
                self.food = pos;
                break;
            }
        }
    }
}

// Screensaver - Matrix rain effect
#[wasm_bindgen]
pub struct MatrixScreensaver {
    #[allow(dead_code)]
    width: u32,
    height: u32,
    columns: Vec<MatrixColumn>,
}

struct MatrixColumn {
    x: u32,
    y: i32,
    speed: f32,
    chars: Vec<char>,
}

#[wasm_bindgen]
impl MatrixScreensaver {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        web_sys::console::log_1(&format!("MatrixScreensaver::new({}x{})", width, height).into());
        let num_columns = width / 12;
        let mut columns = Vec::new();

        for i in 0..num_columns {
            columns.push(MatrixColumn {
                x: i * 12,
                y: -(js_sys::Math::random() * height as f64) as i32,
                speed: 2.0 + js_sys::Math::random() as f32 * 3.0,
                chars: Self::random_chars(20),
            });
        }

        web_sys::console::log_1(&format!("Created {} columns", num_columns).into());
        MatrixScreensaver {
            width,
            height,
            columns,
        }
    }

    pub fn update(&mut self) {
        for col in &mut self.columns {
            col.y += col.speed as i32;
            if col.y > self.height as i32 + 100 {
                col.y = -(js_sys::Math::random() * 200.0) as i32;
                col.speed = 2.0 + js_sys::Math::random() as f32 * 3.0;
                col.chars = Self::random_chars(20);
            }
        }
    }

    pub fn render(&self, gfx: &mut Graphics) {
        // Fade effect - darken existing pixels
        let gfx_width = gfx.width();
        let gfx_height = gfx.height();

        for y in 0..gfx_height {
            for x in 0..gfx_width {
                // Use safe get/set methods instead of direct buffer access
                let idx = ((y * gfx.buffer.width + x) * 4) as usize;
                if idx + 3 < gfx.buffer.pixels.len() && gfx.buffer.pixels[idx + 1] > 5 {
                    gfx.buffer.pixels[idx + 1] -= 5;
                }
            }
        }

        // Draw columns (simplified text rendering using rectangles)
        for col in &self.columns {
            for (i, &ch) in col.chars.iter().enumerate() {
                let y = col.y + (i as i32 * 12);
                if y < 0 || y >= gfx_height as i32 {
                    continue;
                }
                let y_u32 = y as u32;

                let brightness = if i == 0 {
                    255
                } else {
                    180u8.saturating_sub(i as u8 * 8)
                };
                // Draw character representation as small blocks
                let pattern = ch as u32 % 16;
                for dy in 0..10 {
                    let py = y_u32.saturating_add(dy);
                    if py >= gfx_height {
                        continue;
                    }
                    for dx in 0..8 {
                        if (pattern & (1 << (dx % 4))) != 0 {
                            let px = col.x.saturating_add(dx);
                            if px < gfx_width {
                                gfx.set_pixel(px, py, 0, brightness, 0);
                            }
                        }
                    }
                }
            }
        }
    }

    fn random_chars(count: usize) -> Vec<char> {
        let chars = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ日本語ﾊﾝｶｸｶﾅ";
        (0..count)
            .map(|_| {
                let idx = (js_sys::Math::random() * chars.len() as f64) as usize;
                chars.chars().nth(idx).unwrap_or('0')
            })
            .collect()
    }
}
