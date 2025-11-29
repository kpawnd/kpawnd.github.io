#[cfg(feature = "webgl")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "webgl")]
use wasm_bindgen::JsCast;
#[cfg(feature = "webgl")]
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlRenderingContext, WebGlShader,
    WebGlTexture,
};

#[cfg(feature = "webgl")]
pub struct WebGlGraphics {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext, // fall back manually if needed
    program: WebGlProgram,
    texture: WebGlTexture,
    width: u32,
    height: u32,
    /// CPU side pixel buffer (RGBA8)
    pixels: Vec<u8>,
}

#[cfg(feature = "webgl")]
fn get_canvas(id: &str, width: u32, height: u32) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let doc = window.document().ok_or("no document")?;
    let el = doc.get_element_by_id(id).ok_or("canvas not found")?;
    let canvas: HtmlCanvasElement = el.dyn_into()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Ok(canvas)
}

#[cfg(feature = "webgl")]
fn compile_shader(gl: &WebGl2RenderingContext, ty: u32, src: &str) -> Result<WebGlShader, JsValue> {
    let shader = gl.create_shader(ty).ok_or("shader alloc")?;
    gl.shader_source(&shader, src);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&shader).unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "webgl")]
fn link_program(
    gl: &WebGl2RenderingContext,
    vs: &WebGlShader,
    fs: &WebGlShader,
) -> Result<WebGlProgram, JsValue> {
    let program = gl.create_program().ok_or("program alloc")?;
    gl.attach_shader(&program, vs);
    gl.attach_shader(&program, fs);
    gl.link_program(&program);
    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(JsValue::from_str(
            &gl.get_program_info_log(&program).unwrap_or_default(),
        ))
    }
}

#[cfg(feature = "webgl")]
impl WebGlGraphics {
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Result<Self, JsValue> {
        let canvas = get_canvas(canvas_id, width, height)?;
        // Attempt WebGL2 if unavailable fallback to WebGL1 via cast
        let gl2 = canvas
            .get_context("webgl2")?
            .ok_or("webgl2 context not available")?
            .dyn_into::<WebGl2RenderingContext>()?;
        let vertex_src = r#"#version 300 es
        precision mediump float;
        in vec2 aPos;
        in vec2 aUv;
        out vec2 vUv;
        void main() {
            vUv = aUv;
            gl_Position = vec4(aPos, 0.0, 1.0);
        }
        "#;
        let fragment_src = r#"#version 300 es
        precision mediump float;
        in vec2 vUv;
        out vec4 color;
        uniform sampler2D uTex;
        void main() {
            color = texture(uTex, vUv);
        }
        "#;
        let vs = compile_shader(&gl2, WebGl2RenderingContext::VERTEX_SHADER, vertex_src)?;
        let fs = compile_shader(&gl2, WebGl2RenderingContext::FRAGMENT_SHADER, fragment_src)?;
        let program = link_program(&gl2, &vs, &fs)?;
        gl2.use_program(Some(&program));

        // Create fullscreen quad (two triangles) with interleaved pos/uv
        let verts: [f32; 24] = [
            // x,y, u,v
            -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 0.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];
        let vbo = gl2.create_buffer().ok_or("vbo")?;
        gl2.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let vert_bytes = js_sys::Float32Array::view(&verts);
            gl2.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_bytes,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        let mut offset = 0;
        let stride = 4 * std::mem::size_of::<f32>() as i32;
        let pos_loc = gl2.get_attrib_location(&program, "aPos");
        gl2.enable_vertex_attrib_array(pos_loc as u32);
        gl2.vertex_attrib_pointer_with_i32(
            pos_loc as u32,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            stride,
            offset,
        );
        offset += 2 * std::mem::size_of::<f32>() as i32;
        let uv_loc = gl2.get_attrib_location(&program, "aUv");
        gl2.enable_vertex_attrib_array(uv_loc as u32);
        gl2.vertex_attrib_pointer_with_i32(
            uv_loc as u32,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            stride,
            offset,
        );

        // Texture
        let texture = gl2.create_texture().ok_or("texture")?;
        gl2.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        gl2.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        gl2.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::NEAREST as i32,
        );
        gl2.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            width as i32,
            height as i32,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            None,
        )?;
        let pixels = vec![0u8; (width * height * 4) as usize];
        Ok(Self {
            canvas,
            gl: gl2,
            program,
            texture,
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx] = r;
        self.pixels[idx + 1] = g;
        self.pixels[idx + 2] = b;
        self.pixels[idx + 3] = 255;
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = 255;
        }
    }

    pub fn present(&mut self) -> Result<(), JsValue> {
        self.upload_and_draw()
    }

    pub fn upload_and_draw(&self) -> Result<(), JsValue> {
        self.gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
        // SAFETY: pixels slice lives for duration of this call
        let view = unsafe { js_sys::Uint8Array::view(&self.pixels) };
        self.gl
            .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                0,
                0,
                self.width as i32,
                self.height as i32,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(&view),
            )?;
        self.gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        if width == self.width && height == self.height {
            return Ok(());
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.width = width;
        self.height = height;
        self.pixels.resize((width * height * 4) as usize, 0);
        self.gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.texture));
        self.gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGl2RenderingContext::TEXTURE_2D,
                0,
                WebGl2RenderingContext::RGBA as i32,
                width as i32,
                height as i32,
                0,
                WebGl2RenderingContext::RGBA,
                WebGl2RenderingContext::UNSIGNED_BYTE,
                None,
            )?;
        Ok(())
    }
}
