use crate::{
    gl,
    graphic::color::{Rgb, Rgba},
};
use std::ffi::CString;

static mut SHADER_PROGRAM: u32 = 0;
static mut VAO: u32 = 0;
static mut VBO: u32 = 0;
static mut INITIALIZED: bool = false;

const VERTEX_SHADER: &str = r#"
#version 150 core
in vec2 position;
uniform vec4 color;
out vec4 fragColor;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    fragColor = color;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 150 core
in vec4 fragColor;
out vec4 outColor;
void main() {
    outColor = fragColor;
}
"#;

unsafe fn compile_shader(type_: u32, source: &str) -> u32 {
    let shader = gl::CreateShader(type_);
    let c_str = CString::new(source.as_bytes()).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);

    let mut success = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success == 0 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = vec![0u8; len as usize];
        gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut i8,
        );
        log::error!("Shader compile error: {}", String::from_utf8_lossy(&buffer));
    }
    shader
}

unsafe fn setup_opengl() {
    let vs = compile_shader(gl::VERTEX_SHADER, VERTEX_SHADER);
    let fs = compile_shader(gl::FRAGMENT_SHADER, FRAGMENT_SHADER);

    SHADER_PROGRAM = gl::CreateProgram();
    gl::AttachShader(SHADER_PROGRAM, vs);
    gl::AttachShader(SHADER_PROGRAM, fs);
    gl::LinkProgram(SHADER_PROGRAM);

    let mut success = 0;
    gl::GetProgramiv(SHADER_PROGRAM, gl::LINK_STATUS, &mut success);
    if success == 0 {
        let mut len = 0;
        gl::GetProgramiv(SHADER_PROGRAM, gl::INFO_LOG_LENGTH, &mut len);
        let mut buffer = vec![0u8; len as usize];
        gl::GetProgramInfoLog(
            SHADER_PROGRAM,
            len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut i8,
        );
        log::error!("Program link error: {}", String::from_utf8_lossy(&buffer));
    }

    gl::GenVertexArrays(1, std::ptr::addr_of_mut!(VAO));
    gl::GenBuffers(1, std::ptr::addr_of_mut!(VBO));

    gl::BindVertexArray(VAO);
    gl::BindBuffer(gl::ARRAY_BUFFER, VBO);

    // Pre-allocate buffer for 4 vertices (2 floats each)
    gl::BufferData(
        gl::ARRAY_BUFFER,
        (4 * 2 * std::mem::size_of::<f32>()) as isize,
        std::ptr::null(),
        gl::DYNAMIC_DRAW,
    );

    let pos_loc = gl::GetAttribLocation(SHADER_PROGRAM, CString::new("position").unwrap().as_ptr());
    if pos_loc >= 0 {
        gl::VertexAttribPointer(pos_loc as u32, 2, gl::FLOAT, gl::FALSE, 8, std::ptr::null());
        gl::EnableVertexAttribArray(pos_loc as u32);
    }

    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::BindVertexArray(0);

    INITIALIZED = true;
}

/// A state-restoring OpenGL renderer that supports 2D drawing with alpha blending natively.
pub struct Renderer {
    pub screen_width: i32,
    pub screen_height: i32,
    old_blend_src_rgb: i32,
    old_blend_dst_rgb: i32,
    old_blend_src_alpha: i32,
    old_blend_dst_alpha: i32,
    is_blend_on: bool,
    old_program: i32,
    old_vao: i32,
    old_vbo: i32,
    old_depth_test: bool,
    old_cull_face: bool,
    old_scissor_test: bool,
    current_color: Rgba,
}

impl Renderer {
    pub unsafe fn new() -> Self {
        if !INITIALIZED {
            setup_opengl();
        }

        let mut viewport = [0; 4];
        gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        let screen_width = viewport[2];
        let screen_height = viewport[3];

        // Backup states
        let is_blend_on = gl::IsEnabled(gl::BLEND) == gl::TRUE;
        let mut old_blend_src_rgb = 0;
        let mut old_blend_dst_rgb = 0;
        let mut old_blend_src_alpha = 0;
        let mut old_blend_dst_alpha = 0;
        gl::GetIntegerv(gl::BLEND_SRC_RGB, &mut old_blend_src_rgb);
        gl::GetIntegerv(gl::BLEND_DST_RGB, &mut old_blend_dst_rgb);
        gl::GetIntegerv(gl::BLEND_SRC_ALPHA, &mut old_blend_src_alpha);
        gl::GetIntegerv(gl::BLEND_DST_ALPHA, &mut old_blend_dst_alpha);

        let mut old_program = 0;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut old_program);
        let mut old_vao = 0;
        gl::GetIntegerv(gl::VERTEX_ARRAY_BINDING, &mut old_vao);
        let mut old_vbo = 0;
        gl::GetIntegerv(gl::ARRAY_BUFFER_BINDING, &mut old_vbo);

        let old_depth_test = gl::IsEnabled(gl::DEPTH_TEST) == gl::TRUE;
        let old_cull_face = gl::IsEnabled(gl::CULL_FACE) == gl::TRUE;
        let old_scissor_test = gl::IsEnabled(gl::SCISSOR_TEST) == gl::TRUE;

        // Apply our states
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);

        gl::UseProgram(SHADER_PROGRAM);
        gl::BindVertexArray(VAO);
        // Ensure VBO is bound for drawing
        gl::BindBuffer(gl::ARRAY_BUFFER, VBO);

        Self {
            screen_width,
            screen_height,
            is_blend_on,
            old_blend_src_rgb,
            old_blend_dst_rgb,
            old_blend_src_alpha,
            old_blend_dst_alpha,
            old_program,
            old_vao,
            old_vbo,
            old_depth_test,
            old_cull_face,
            old_scissor_test,
            current_color: Rgba::new(Rgb::new(1.0, 1.0, 1.0), 1.0),
        }
    }

    pub unsafe fn set_color(&mut self, rgba: Rgba) {
        self.current_color = rgba;
    }

    pub unsafe fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let sc_w = self.screen_width as f32;
        let sc_h = self.screen_height as f32;

        let ndc_x1 = (x as f32 / sc_w) * 2.0 - 1.0;
        let ndc_y1 = 1.0 - (y as f32 / sc_h) * 2.0;
        let ndc_x2 = ((x + w) as f32 / sc_w) * 2.0 - 1.0;
        let ndc_y2 = 1.0 - ((y + h) as f32 / sc_h) * 2.0;

        let vertices: [f32; 8] = [
            ndc_x1, ndc_y1, // Top-left
            ndc_x1, ndc_y2, // Bottom-left
            ndc_x2, ndc_y1, // Top-right
            ndc_x2, ndc_y2, // Bottom-right
        ];

        // Send new vertices to VBO
        gl::BufferSubData(
            gl::ARRAY_BUFFER,
            0,
            (vertices.len() * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as *const _,
        );

        // Upload uniform color
        let color_loc =
            gl::GetUniformLocation(SHADER_PROGRAM, CString::new("color").unwrap().as_ptr());
        if color_loc >= 0 {
            gl::Uniform4f(
                color_loc,
                self.current_color.r,
                self.current_color.g,
                self.current_color.b,
                self.current_color.a,
            );
        }

        // Draw as Triangle Strip (4 vertices make 2 triangles / 1 quad)
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
    }

    pub unsafe fn enable_scissor(&self, x: i32, y: i32, w: i32, h: i32) {
        gl::Enable(gl::SCISSOR_TEST);
        let bottom_y = self.screen_height - (y + h);
        gl::Scissor(x, bottom_y, w, h);
    }

    pub unsafe fn disable_scissor(&self) {
        gl::Disable(gl::SCISSOR_TEST);
    }

    pub unsafe fn draw_quad(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
    ) {
        let sc_w = self.screen_width as f32;
        let sc_h = self.screen_height as f32;

        let ndc_x1 = (x1 / sc_w) * 2.0 - 1.0;
        let ndc_y1 = 1.0 - (y1 / sc_h) * 2.0;

        let ndc_x2 = (x2 / sc_w) * 2.0 - 1.0;
        let ndc_y2 = 1.0 - (y2 / sc_h) * 2.0;

        let ndc_x3 = (x3 / sc_w) * 2.0 - 1.0;
        let ndc_y3 = 1.0 - (y3 / sc_h) * 2.0;

        let ndc_x4 = (x4 / sc_w) * 2.0 - 1.0;
        let ndc_y4 = 1.0 - (y4 / sc_h) * 2.0;

        let vertices: [f32; 8] = [
            ndc_x1, ndc_y1, // Top-left
            ndc_x3, ndc_y3, // Bottom-left
            ndc_x2, ndc_y2, // Top-right
            ndc_x4, ndc_y4, // Bottom-right
        ];

        // Send new vertices to VBO
        gl::BufferSubData(
            gl::ARRAY_BUFFER,
            0,
            (vertices.len() * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as *const _,
        );

        // Upload uniform color
        let color_loc =
            gl::GetUniformLocation(SHADER_PROGRAM, CString::new("color").unwrap().as_ptr());
        if color_loc >= 0 {
            gl::Uniform4f(
                color_loc,
                self.current_color.r,
                self.current_color.g,
                self.current_color.b,
                self.current_color.a,
            );
        }

        // Draw as Triangle Strip
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            if self.old_depth_test {
                gl::Enable(gl::DEPTH_TEST);
            }
            if self.old_cull_face {
                gl::Enable(gl::CULL_FACE);
            }
            if !self.old_scissor_test {
                gl::Disable(gl::SCISSOR_TEST);
            } else {
                gl::Enable(gl::SCISSOR_TEST);
            }

            if !self.is_blend_on {
                gl::Disable(gl::BLEND);
            } else {
                gl::BlendFuncSeparate(
                    self.old_blend_src_rgb as u32,
                    self.old_blend_dst_rgb as u32,
                    self.old_blend_src_alpha as u32,
                    self.old_blend_dst_alpha as u32,
                );
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, self.old_vbo as u32);
            gl::BindVertexArray(self.old_vao as u32);
            gl::UseProgram(self.old_program as u32);
        }
    }
}
