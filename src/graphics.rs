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

    /// Draw a filled circle using midpoint circle algorithm
    pub fn fill_circle(&mut self, cx: u32, cy: u32, radius: u32, color: &Color) {
        if radius == 0 {
            return;
        }

        let r = radius as i32;
        let mut x = 0;
        let mut y = r;
        let mut d = 3 - 2 * r;

        while y >= x {
            // Draw horizontal lines to fill the circle
            let y_pos = cy as i32 + y;
            let y_neg = cy as i32 - y;
            let y_pos2 = cy as i32 + x;
            let y_neg2 = cy as i32 - x;

            let x_left = (cx as i32 - x).max(0) as u32;
            let x_right = (cx as i32 + x).min(self.width as i32 - 1) as u32;

            // Fill the horizontal spans
            if y_pos >= 0 && y_pos < self.height as i32 {
                self.draw_hline(x_left, x_right, y_pos as u32, color.r, color.g, color.b);
            }
            if y_neg >= 0 && y_neg < self.height as i32 {
                self.draw_hline(x_left, x_right, y_neg as u32, color.r, color.g, color.b);
            }

            if x != y {
                let x_left2 = (cx as i32 - y).max(0) as u32;
                let x_right2 = (cx as i32 + y).min(self.width as i32 - 1) as u32;

                if y_pos2 >= 0 && y_pos2 < self.height as i32 {
                    self.draw_hline(x_left2, x_right2, y_pos2 as u32, color.r, color.g, color.b);
                }
                if y_neg2 >= 0 && y_neg2 < self.height as i32 {
                    self.draw_hline(x_left2, x_right2, y_neg2 as u32, color.r, color.g, color.b);
                }
            }

            x += 1;
            if d > 0 {
                y -= 1;
                d += 4 * (x - y) + 10;
            } else {
                d += 4 * x + 6;
            }
        }
    }

    /// Draw a triangle (outline)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_triangle(
        &mut self,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        x3: u32,
        y3: u32,
        color: &Color,
    ) {
        self.draw_line(x1, y1, x2, y2, color);
        self.draw_line(x2, y2, x3, y3, color);
        self.draw_line(x3, y3, x1, y1, color);
    }

    /// Draw a filled triangle using barycentric coordinates
    #[allow(clippy::too_many_arguments)]
    pub fn fill_triangle(
        &mut self,
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        x3: u32,
        y3: u32,
        color: &Color,
    ) {
        // Find bounding box
        let min_x = x1.min(x2).min(x3);
        let max_x = x1.max(x2).max(x3);
        let min_y = y1.min(y2).min(y3);
        let max_y = y1.max(y2).max(y3);

        // Convert to i32 for calculations
        let x1 = x1 as i32;
        let y1 = y1 as i32;
        let x2 = x2 as i32;
        let y2 = y2 as i32;
        let x3 = x3 as i32;
        let y3 = y3 as i32;

        // Iterate over bounding box
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let x = x as i32;
                let y = y as i32;

                // Check if point is inside triangle using barycentric coordinates
                let denom = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);
                if denom == 0 {
                    continue;
                }

                let a = ((y2 - y3) * (x - x3) + (x3 - x2) * (y - y3)) as f64 / denom as f64;
                let b = ((y3 - y1) * (x - x3) + (x1 - x3) * (y - y3)) as f64 / denom as f64;
                let c = 1.0 - a - b;

                if (0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b) && (0.0..=1.0).contains(&c)
                {
                    self.set_pixel(x as u32, y as u32, color);
                }
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
    width: u32,
    height: u32,
    cell_w: u32,
    cell_h: u32,
    frame: u64,
    columns: Vec<MatrixColumn>,
}

struct MatrixColumn {
    x: u32,
    head_y: f32,
    speed: f32,
    length: u32,
    glyph_phase: u32,
}

#[wasm_bindgen]
impl MatrixScreensaver {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        let cell_w = 10;
        let cell_h = 16;
        let num_columns = (width / cell_w).max(1);
        let mut columns = Vec::with_capacity(num_columns as usize);

        for i in 0..num_columns {
            columns.push(Self::new_column(i, cell_w, width, height));
        }

        Self {
            width,
            height,
            cell_w,
            cell_h,
            frame: 0,
            columns,
        }
    }

    pub fn update(&mut self) {
        self.frame = self.frame.wrapping_add(1);

        for col in &mut self.columns {
            col.head_y += col.speed;

            // Rare drift creates the "digital rain" cadence seen in cmatrix.
            if js_sys::Math::random() < 0.012 {
                col.speed = (2.8 + js_sys::Math::random() as f32 * 9.0).min(15.0);
            }
            if js_sys::Math::random() < 0.008 {
                col.length = 8 + (js_sys::Math::random() * 32.0) as u32;
            }

            let trail_px = (col.length as i32 * self.cell_h as i32) + (self.cell_h as i32 * 2);
            if col.head_y as i32 - trail_px > self.height as i32 {
                let x_slot = (col.x / self.cell_w) as usize;
                *col = Self::new_column(x_slot as u32, self.cell_w, self.width, self.height);
            }
        }
    }

    pub fn render(&self, gfx: &mut Graphics) {
        // Fade effect keeps phosphor-like trails.
        let gfx_height = gfx.height();
        crate::cpp_accel::fade_rgba_sub(&mut gfx.buffer.pixels, 10, 16, 10);

        for col in &self.columns {
            let max_steps = col.length as i32;
            for step in 0..max_steps {
                let y = col.head_y as i32 - (step * self.cell_h as i32);
                if y < -(self.cell_h as i32) || y >= gfx_height as i32 {
                    continue;
                }

                let is_head = step == 0;
                let intensity = if is_head {
                    255
                } else {
                    let falloff = (step as f32 / max_steps.max(1) as f32).min(1.0);
                    (210.0 * (1.0 - falloff)).max(20.0) as u8
                };

                let (r, g, b) = if is_head {
                    (215, 255, 215)
                } else {
                    (0, intensity, 0)
                };

                let glyph_seed = self
                    .frame
                    .wrapping_add((col.glyph_phase as u64).wrapping_mul(7919))
                    .wrapping_add((step as u64).wrapping_mul(97))
                    .wrapping_add(col.x as u64 * 17);

                self.draw_glyph(gfx, col.x, y as u32, glyph_seed as u32, r, g, b);
            }
        }
    }

    fn new_column(slot: u32, cell_w: u32, width: u32, height: u32) -> MatrixColumn {
        let mut x = slot.saturating_mul(cell_w);
        if cell_w > 2 {
            x = x.saturating_add((js_sys::Math::random() * (cell_w - 2) as f64) as u32);
        }
        x = x.min(width.saturating_sub(1));

        MatrixColumn {
            x,
            head_y: -((js_sys::Math::random() * height as f64) as f32),
            speed: 2.8 + js_sys::Math::random() as f32 * 9.0,
            length: 8 + (js_sys::Math::random() * 28.0) as u32,
            glyph_phase: (js_sys::Math::random() * u32::MAX as f64) as u32,
        }
    }

    fn draw_glyph(&self, gfx: &mut Graphics, x: u32, y: u32, seed: u32, r: u8, g: u8, b: u8) {
        // Synthetic 5x7 glyphs with slight variance approximate cmatrix character shimmer.
        let scale_x = (self.cell_w / 5).max(1);
        let scale_y = (self.cell_h / 8).max(1);
        let glyph = Self::glyph_from_seed(seed);

        for row in 0..7u32 {
            let bits = glyph[row as usize];
            for col in 0..5u32 {
                if (bits >> (4 - col)) & 1 == 0 {
                    continue;
                }

                let px = x.saturating_add(col * scale_x);
                let py = y.saturating_add(row * scale_y);
                for dy in 0..scale_y {
                    for dx in 0..scale_x {
                        let tx = px.saturating_add(dx);
                        let ty = py.saturating_add(dy);
                        if tx < gfx.width() && ty < gfx.height() {
                            gfx.set_pixel(tx, ty, r, g, b);
                        }
                    }
                }
            }
        }
    }

    fn glyph_from_seed(seed: u32) -> [u8; 7] {
        let mut s = seed ^ 0x9e37_79b9;
        let mut rows = [0u8; 7];
        for row in &mut rows {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            let mut bits = (s & 0x1f) as u8;
            // Ensure at least 2 pixels are lit so glyphs stay legible.
            if bits.count_ones() < 2 {
                bits |= 0b10001;
            }
            *row = bits;
        }
        rows
    }
}
