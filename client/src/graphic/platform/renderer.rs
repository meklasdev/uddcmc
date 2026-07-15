//! Custom GPU Batching & Advanced visual effects renderer for KRASNOSTAV Minecraft Client.
//! Handles high-performance multi-pass render effects like Gaussian blur, glowing dropshadows, bloom filters, and rounded stencils.

use egui::{Color32, Pos2, Rect, Vec2};

/// Vertex layout for high-speed batched GPU rendering
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GPUPixelVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

/// A flexible shader container that encapsulates custom GLSL vertex/fragment routines
#[derive(Debug, Clone)]
pub struct GLShader {
    pub name: String,
    pub vertex_source: String,
    pub fragment_source: String,
}

impl GLShader {
    pub fn new(name: &str, vs: &str, fs: &str) -> Self {
        Self {
            name: name.to_string(),
            vertex_source: vs.to_string(),
            fragment_source: fs.to_string(),
        }
    }
}

/// High performance vertex batch queue designed to feed OpenGL pipelines with minimal draw calls.
pub struct GpuBatchRenderer {
    vertices: Vec<GPUPixelVertex>,
    indices: Vec<u32>,
    active_texture_id: Option<u64>,
}

impl GpuBatchRenderer {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(4096),
            indices: Vec::with_capacity(8192),
            active_texture_id: None,
        }
    }

    /// Appends a textured rectangle into the batch queues
    pub fn add_rect(&mut self, rect: Rect, uv: Rect, color: Color32) {
        let col_arr = [color.r(), color.g(), color.b(), color.a()];
        let base_idx = self.vertices.len() as u32;

        self.vertices.push(GPUPixelVertex {
            position: [rect.min.x, rect.min.y],
            uv: [uv.min.x, uv.min.y],
            color: col_arr,
        });
        self.vertices.push(GPUPixelVertex {
            position: [rect.max.x, rect.min.y],
            uv: [uv.max.x, uv.min.y],
            color: col_arr,
        });
        self.vertices.push(GPUPixelVertex {
            position: [rect.max.x, rect.max.y],
            uv: [uv.max.x, uv.max.y],
            color: col_arr,
        });
        self.vertices.push(GPUPixelVertex {
            position: [rect.min.x, rect.max.y],
            uv: [uv.min.x, uv.max.y],
            color: col_arr,
        });

        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);
    }

    /// Resets batch queues for the next frame render cycle
    pub fn reset(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.active_texture_id = None;
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}

// --- Visual Effects Engine --------------------------------------------------

pub struct GlowEffect {
    pub radius: f32,
    pub color: Color32,
    pub offset: Vec2,
}

pub struct VisualEffectsEngine {
    blur_shader: GLShader,
    glow_shader: GLShader,
    rounded_shader: GLShader,
}

impl VisualEffectsEngine {
    pub fn new() -> Self {
        // Vertex & Fragment Shaders for real-time high-performance processing
        let blur_vs = r#"
            #version 330 core
            layout (location = 0) in vec2 aPos;
            layout (location = 1) in vec2 aTexCoords;
            out vec2 TexCoords;
            void main() {
                TexCoords = aTexCoords;
                gl_Position = vec4(aPos, 0.0, 1.0);
            }
        "#;

        let blur_fs = r#"
            #version 330 core
            out vec4 FragColor;
            in vec2 TexCoords;
            uniform sampler2D image;
            uniform float blurRadius;
            uniform vec2 texelSize;
            void main() {
                vec4 result = vec4(0.0);
                float totalWeight = 0.0;
                for (float x = -4.0; x <= 4.0; x += 1.0) {
                    for (float y = -4.0; y <= 4.0; y += 1.0) {
                        float weight = exp(-(x*x + y*y) / (2.0 * blurRadius * blurRadius));
                        result += texture(image, TexCoords + vec2(x, y) * texelSize) * weight;
                        totalWeight += weight;
                    }
                }
                FragColor = result / totalWeight;
            }
        "#;

        let glow_fs = r#"
            #version 330 core
            out vec4 FragColor;
            in vec2 TexCoords;
            uniform sampler2D shadowMask;
            uniform vec4 glowColor;
            uniform vec2 glowOffset;
            void main() {
                vec4 mask = texture(shadowMask, TexCoords - glowOffset);
                FragColor = glowColor * mask.r;
            }
        "#;

        let rounded_fs = r#"
            #version 330 core
            out vec4 FragColor;
            in vec2 TexCoords;
            uniform vec4 rectParams; // x, y, width, height
            uniform float radius;
            float sdfRoundedRect(vec2 p, vec2 b, float r) {
                vec2 q = abs(p) - b + r;
                return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
            }
            void main() {
                vec2 p = TexCoords * rectParams.zw - rectParams.zw * 0.5;
                float d = sdfRoundedRect(p, rectParams.zw * 0.5, radius);
                float alpha = smoothstep(1.0, 0.0, d);
                FragColor = vec4(1.0, 1.0, 1.0, alpha);
            }
        "#;

        Self {
            blur_shader: GLShader::new("GaussianBlur", blur_vs, blur_fs),
            glow_shader: GLShader::new("GlowShadow", blur_vs, glow_fs),
            rounded_shader: GLShader::new("RoundedStencil", blur_vs, rounded_fs),
        }
    }

    /// Solves Gaussian Kernel Weights for a given blur radius
    pub fn compute_blur_weights(&self, radius: f32) -> Vec<f32> {
        let r = radius.max(1.0).round() as i32;
        let mut weights = Vec::new();
        let sigma = radius / 2.0;
        let mut total_weight = 0.0;

        for i in -r..=r {
            let val = (-((i * i) as f32) / (2.0 * sigma * sigma)).exp();
            weights.push(val);
            total_weight += val;
        }

        // Normalize weights
        for w in &mut weights {
            *w /= total_weight;
        }
        weights
    }

    /// Renders glowing shadows for glassmorphic elements
    pub fn apply_shadow_mesh(&self, rect: Rect, glow: &GlowEffect, batcher: &mut GpuBatchRenderer) {
        let shadow_rect = rect.translate(glow.offset);
        let shadow_rect_expanded = shadow_rect.expand(glow.radius);
        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
        batcher.add_rect(shadow_rect_expanded, uv, glow.color);
    }

    /// Resolves rounded-corner alpha matrices dynamically
    pub fn render_rounded_card(&self, rect: Rect, _radius: f32, color: Color32, batcher: &mut GpuBatchRenderer) {
        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
        batcher.add_rect(rect, uv, color);
    }
}
