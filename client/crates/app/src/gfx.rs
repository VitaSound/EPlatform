//! wgpu surface on the winit window: configure, clear, present.
//!
//! Display list: solid rects, SDF shapes (gradient / rounded / ellipse),
//! textured quads, and glyphon text — replayed in painter order.

use std::collections::HashMap;

use winit::window::Window;

use glyphon::{
    Attrs, Buffer, Cache as GlyphCache, Color, Family, FontSystem, Metrics, PrepareError,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

/// Soft caps — buffers grow if a frame needs more (200 MDI windows ≈
/// tens of thousands of solid fills; diagonal chrome icons used to blow
/// past a fixed 8k via 1×1 pixel floods).
const INITIAL_RECTS: usize = 16384;
const INITIAL_SHAPES: usize = 8192;
const INITIAL_TEXTURED: usize = 1024;

const RECT_WGSL: &str = r#"
struct Out {
    @builtin(position) pos: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs(@location(0) p: vec2f, @location(1) c: vec4f) -> Out {
    var out: Out;
    out.pos = vec4f(p, 0.0, 1.0);
    out.color = c;
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4f {
    return in.color;
}
"#;

/// SDF shapes: solid/rounded fill, linear gradient (+radius), ellipse, segment,
/// dashed stroke rect.
/// `params = (radius, stroke_width, kind, _)` where kind is
/// 0=solid, 1=gradientH, 2=gradientV, 3=ellipse, 4=line (endpoints in color1),
/// 5=dashed stroke (dash/gap in color1.xy).
const SHAPE_WGSL: &str = r#"
struct Out {
    @builtin(position) pos: vec4f,
    @location(0) local: vec2f,
    @location(1) half_size: vec2f,
    @location(2) color0: vec4f,
    @location(3) color1: vec4f,
    @location(4) params: vec4f,
};

@vertex
fn vs(
    @location(0) p: vec2f,
    @location(1) local: vec2f,
    @location(2) half_size: vec2f,
    @location(3) color0: vec4f,
    @location(4) color1: vec4f,
    @location(5) params: vec4f,
) -> Out {
    var out: Out;
    out.pos = vec4f(p, 0.0, 1.0);
    out.local = local;
    out.half_size = half_size;
    out.color0 = color0;
    out.color1 = color1;
    out.params = params;
    return out;
}

fn sd_rounded_box(p: vec2f, b: vec2f, r: f32) -> f32 {
    let rr = min(r, min(b.x, b.y));
    let q = abs(p) - b + vec2f(rr, rr);
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - rr;
}

fn sd_ellipse(p: vec2f, r: vec2f) -> f32 {
    let safe = max(r, vec2f(0.5, 0.5));
    return (length(p / safe) - 1.0) * min(safe.x, safe.y);
}

fn sd_segment(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-4), 0.0, 1.0);
    return length(pa - ba * h);
}

/// Arc length along the rectangle perimeter (top→right→bottom→left).
fn rect_outline_param(p: vec2f, b: vec2f) -> f32 {
    let hw = max(b.x, 0.5);
    let hh = max(b.y, 0.5);
    let w = hw * 2.0;
    let h = hh * 2.0;
    let ax = abs(p.x) / hw;
    let ay = abs(p.y) / hh;
    if (ax > ay) {
        let y_along = clamp(p.y + hh, 0.0, h);
        if (p.x >= 0.0) {
            return w + y_along;
        }
        return w + h + w + (h - y_along);
    }
    let x_along = clamp(p.x + hw, 0.0, w);
    if (p.y < 0.0) {
        return x_along;
    }
    return w + h + (w - x_along);
}

@fragment
fn fs(in: Out) -> @location(0) vec4f {
    let radius = in.params.x;
    let stroke = in.params.y;
    let kind = in.params.z;

    var d: f32;
    var cover: f32;
    if (kind > 3.5 && kind < 4.5) {
        // color1 = local endpoints (not a color). Capsule around the segment.
        d = sd_segment(in.local, in.color1.xy, in.color1.zw) - max(stroke, 1.0) * 0.5;
        cover = 1.0 - smoothstep(-0.75, 0.75, d);
    } else if (kind > 4.5) {
        d = sd_rounded_box(in.local, in.half_size, radius);
        let outer = 1.0 - smoothstep(-0.75, 0.75, d);
        let inner = 1.0 - smoothstep(-0.75, 0.75, d + max(stroke, 1.0));
        cover = clamp(outer - inner, 0.0, 1.0);
        let dash = max(in.color1.x, 1.0);
        let gap = max(in.color1.y, 1.0);
        let period = dash + gap;
        let s = rect_outline_param(in.local, in.half_size);
        if (period > 0.0 && (s - floor(s / period) * period) >= dash) {
            cover = 0.0;
        }
    } else {
        if (kind > 2.5) {
            d = sd_ellipse(in.local, in.half_size);
        } else {
            d = sd_rounded_box(in.local, in.half_size, radius);
        }
        if (stroke > 0.0) {
            let outer = 1.0 - smoothstep(-0.75, 0.75, d);
            let inner = 1.0 - smoothstep(-0.75, 0.75, d + stroke);
            cover = clamp(outer - inner, 0.0, 1.0);
        } else {
            cover = 1.0 - smoothstep(-0.75, 0.75, d);
        }
    }
    if (cover <= 0.001) {
        discard;
    }

    var color = in.color0;
    if (kind > 0.5 && kind < 1.5) {
        let span = max(in.half_size.x * 2.0, 1.0);
        let t = clamp((in.local.x + in.half_size.x) / span, 0.0, 1.0);
        color = mix(in.color0, in.color1, t);
    } else if (kind > 1.5 && kind < 2.5) {
        let span = max(in.half_size.y * 2.0, 1.0);
        let t = clamp((in.local.y + in.half_size.y) / span, 0.0, 1.0);
        color = mix(in.color0, in.color1, t);
    }

    return vec4f(color.rgb, color.a * cover);
}
"#;

const TEXTURE_WGSL: &str = r#"
struct Out {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) opacity: f32,
};

@group(0) @binding(0) var samp: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;

@vertex
fn vs(@location(0) p: vec2f, @location(1) uv: vec2f, @location(2) opacity: f32) -> Out {
    var out: Out;
    out.pos = vec4f(p, 0.0, 1.0);
    out.uv = uv;
    out.opacity = opacity;
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4f {
    let c = textureSample(tex, samp, in.uv);
    return vec4f(c.rgb, c.a * in.opacity);
}
"#;

/// One rectangle in pixel coordinates + sRGB-ish color.
#[derive(Debug, Clone, Copy)]
pub struct ColoredRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

/// GPU SDF shape (one quad). Coordinates are logical / CSS px.
#[derive(Debug, Clone, Copy)]
pub struct ShapePrim {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Corner radius (ignored for ellipse).
    pub radius: f32,
    /// `0` = filled; otherwise stroke width in logical px.
    pub stroke: f32,
    /// 0 solid, 1 gradientH, 2 gradientV, 3 ellipse, 4 line segment, 5 dashed stroke.
    pub kind: u8,
    pub color0: [f32; 4],
    /// Gradient end — or for `kind==4`, local endpoints `(ax,ay,bx,by)`;
    /// for `kind==5`, `(dash, gap, _, _)`.
    pub color1: [f32; 4],
}

impl ShapePrim {
    pub fn solid(x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            stroke: 0.0,
            kind: 0,
            color0: color,
            color1: color,
        }
    }

    pub fn gradient(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        start: [f32; 4],
        end: [f32; 4],
        horizontal: bool,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            stroke: 0.0,
            kind: if horizontal { 1 } else { 2 },
            color0: start,
            color1: end,
        }
    }

    pub fn stroke_rounded(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        width: f32,
        color: [f32; 4],
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius,
            stroke: width.max(1.0),
            kind: 0,
            color0: color,
            color1: color,
        }
    }

    pub fn ellipse(x: f32, y: f32, w: f32, h: f32, stroke: f32, color: [f32; 4]) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius: 0.0,
            stroke,
            kind: 3,
            color0: color,
            color1: color,
        }
    }

    /// One stroked segment (chrome "X", maximize box edges that aren't AA).
    pub fn line(from_x: f32, from_y: f32, to_x: f32, to_y: f32, width: f32, color: [f32; 4]) -> Self {
        let half = width.max(1.0) * 0.5;
        let min_x = from_x.min(to_x) - half;
        let min_y = from_y.min(to_y) - half;
        let max_x = from_x.max(to_x) + half;
        let max_y = from_y.max(to_y) + half;
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        Self {
            x: min_x,
            y: min_y,
            w: (max_x - min_x).max(1.0),
            h: (max_y - min_y).max(1.0),
            radius: 0.0,
            stroke: width.max(1.0),
            kind: 4,
            color0: color,
            color1: [from_x - cx, from_y - cy, to_x - cx, to_y - cy],
        }
    }

    /// One dashed axis-aligned stroke rect (focus rings) — avoids 1×1 floods.
    pub fn dashed_stroke(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        width: f32,
        color: [f32; 4],
        dash: f32,
        gap: f32,
    ) -> Self {
        Self {
            x,
            y,
            w,
            h,
            radius: 0.0,
            stroke: width.max(1.0),
            kind: 5,
            color0: color,
            color1: [dash.max(1.0), gap.max(1.0), 0.0, 0.0],
        }
    }
}

/// Textured blit (icons / pixmaps).
#[derive(Debug, Clone)]
pub struct TexturedRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub opacity: f32,
    pub pixmap: rqt::gui::QPixmap,
}

/// Filled triangle in logical px (scrollbar / combo / tree arrows).
#[derive(Debug, Clone, Copy)]
pub struct ColoredTriangle {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub c: [f32; 2],
    pub color: [f32; 4],
}

/// One text run for glyphon: positioned at `(x, y)` (top-left of the
/// line box), clipped to `clip = [left, top, right, bottom]`.
#[derive(Debug, Clone)]
pub struct ColoredText {
    pub x: f32,
    pub y: f32,
    pub clip: [i32; 4],
    /// Wrapping width (widget width minus padding).
    pub wrap_w: f32,
    pub text: String,
    pub color: [f32; 4],
}

/// One homogeneous run of the ordered display list.
#[derive(Debug, Clone)]
pub enum PaintBatch {
    Rects(Vec<ColoredRect>),
    /// Solid-color triangles share the rect shader (3 verts each).
    Triangles(Vec<ColoredTriangle>),
    Shapes(Vec<ShapePrim>),
    Textures(Vec<TexturedRect>),
    Text(Vec<ColoredText>),
}

/// Font metrics for widget text (labels, inputs, buttons).
const FONT_SIZE: f32 = 14.0;
pub(crate) const LINE_HEIGHT: f32 = 18.0;

/// glyphon state: system fonts (bundled DejaVu), atlas, text pipeline.
struct TextState {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    /// One renderer per consecutive text batch. Rendering them in batch
    /// order inside a single render pass preserves the display list.
    renderers: Vec<TextRenderer>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RectVert {
    pos: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShapeVert {
    pos: [f32; 2],
    local: [f32; 2],
    half_size: [f32; 2],
    color0: [f32; 4],
    color1: [f32; 4],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TexVert {
    pos: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    _pad: [f32; 3],
}

const RECT_ATTRS: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 8,
        shader_location: 1,
    },
];

const SHAPE_ATTRS: [wgpu::VertexAttribute; 6] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 16,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 24,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 40,
        shader_location: 4,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 56,
        shader_location: 5,
    },
];

const TEX_ATTRS: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 16,
        shader_location: 2,
    },
];

struct CachedTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

pub struct Gfx {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    max_dim: u32,
    dpr: f32,
    ui_scale: f32,
    rect_pipeline: wgpu::RenderPipeline,
    shape_pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    texture_bind_layout: wgpu::BindGroupLayout,
    texture_sampler: wgpu::Sampler,
    rect_buffer: wgpu::Buffer,
    shape_buffer: wgpu::Buffer,
    tex_buffer: wgpu::Buffer,
    rect_capacity: usize,
    shape_capacity: usize,
    tex_capacity: usize,
    scratch: Vec<RectVert>,
    shape_scratch: Vec<ShapeVert>,
    tex_scratch: Vec<TexVert>,
    texture_cache: HashMap<u64, CachedTexture>,
    text: TextState,
}

fn fit_dimensions(width: u32, height: u32, max_dim: u32) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let max_dim = max_dim.max(1);
    if width <= max_dim && height <= max_dim {
        return (width, height);
    }
    let scale = max_dim as f32 / width.max(height) as f32;
    (
        ((width as f32 * scale) as u32).max(1),
        ((height as f32 * scale) as u32).max(1),
    )
}

fn ui_scale_for(width: u32, height: u32, dpr: f32, config: &wgpu::SurfaceConfiguration) -> f32 {
    let dpr = if dpr > 0.0 { dpr } else { 1.0 };
    let logical_w = width.max(1) as f32 / dpr;
    let logical_h = height.max(1) as f32 / dpr;
    (config.width as f32 / logical_w)
        .min(config.height as f32 / logical_h)
        .max(0.1)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn color_linear(c: [f32; 4]) -> [f32; 4] {
    [
        srgb_to_linear(c[0]),
        srgb_to_linear(c[1]),
        srgb_to_linear(c[2]),
        c[3],
    ]
}

fn pixmap_key(pixmap: &rqt::gui::QPixmap) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixmap.width.hash(&mut hasher);
    pixmap.height.hash(&mut hasher);
    pixmap.pixels.hash(&mut hasher);
    hasher.finish()
}

fn alpha_blend_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

impl Gfx {
    pub async fn new(window: std::sync::Arc<Window>) -> Result<Self, String> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("create_surface: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("no suitable GPU adapter: {e:?}"))?;

        let info = adapter.get_info();
        crate::App::log(format!(
            "gfx adapter: backend={:?}, device={}",
            info.backend, info.device
        ));

        // WebGL2 needs downlevel defaults for most fields, but keep the
        // adapter's real texture size (often 8192/16384). Plain
        // downlevel_webgl2_defaults() clamps max_texture_dimension_2d to 2048.
        let required_limits = if info.backend == wgpu::Backend::Gl {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default()
        };

        let dpr = window.scale_factor() as f32;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: Default::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| format!("request_device: {e:?}"))?;

        let caps = surface.get_capabilities(&adapter);
        let max_dim = device.limits().max_texture_dimension_2d;
        crate::App::log(format!(
            "gfx limits: max_texture_dimension_2d={max_dim} (adapter={})",
            adapter.limits().max_texture_dimension_2d
        ));
        let (width, height) = fit_dimensions(size.width, size.height, max_dim);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            color_space: Default::default(),
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let ui_scale = ui_scale_for(size.width, size.height, dpr, &config);
        surface.configure(&device, &config);

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_WGSL.into()),
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVert>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &RECT_ATTRS,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shape shader"),
            source: wgpu::ShaderSource::Wgsl(SHAPE_WGSL.into()),
        });
        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shape pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ShapeVert>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &SHAPE_ATTRS,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(alpha_blend_target(config.format))],
            }),
            multiview_mask: None,
            cache: None,
        });

        let texture_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture blit layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("texture blit pipeline layout"),
                bind_group_layouts: &[Some(&texture_bind_layout)],
                immediate_size: 0,
            });
        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("texture shader"),
            source: wgpu::ShaderSource::Wgsl(TEXTURE_WGSL.into()),
        });
        let texture_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture pipeline"),
            layout: Some(&texture_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &texture_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexVert>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &TEX_ATTRS,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &texture_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(alpha_blend_target(config.format))],
            }),
            multiview_mask: None,
            cache: None,
        });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture blit sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let rect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect vertices"),
            size: (INITIAL_RECTS * 6 * std::mem::size_of::<RectVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shape_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shape vertices"),
            size: (INITIAL_SHAPES * 6 * std::mem::size_of::<ShapeVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let tex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("texture vertices"),
            size: (INITIAL_TEXTURED * 6 * std::mem::size_of::<TexVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // glyphon: bundled font (wasm has no system fonts), atlas, renderer.
        let cache = GlyphCache::new(&device);
        let font_system = FontSystem::new_with_fonts([glyphon::fontdb::Source::Binary(
            std::sync::Arc::new(rqt::gui::DEJA_VU_SANS),
        )]);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, config.format);
        let viewport = Viewport::new(&device, &cache);
        let renderer =
            TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);
        let text = TextState {
            font_system,
            swash_cache: SwashCache::new(),
            atlas,
            viewport,
            renderers: vec![renderer],
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            max_dim,
            dpr,
            ui_scale,
            rect_pipeline,
            shape_pipeline,
            texture_pipeline,
            texture_bind_layout,
            texture_sampler,
            rect_buffer,
            shape_buffer,
            tex_buffer,
            rect_capacity: INITIAL_RECTS,
            shape_capacity: INITIAL_SHAPES,
            tex_capacity: INITIAL_TEXTURED,
            scratch: Vec::with_capacity(INITIAL_RECTS * 6),
            shape_scratch: Vec::with_capacity(INITIAL_SHAPES * 6),
            tex_scratch: Vec::with_capacity(INITIAL_TEXTURED * 6),
            texture_cache: HashMap::new(),
            text,
        })
    }

    /// `verts` is the number of `RectVert` entries (rects use 6, triangles 3).
    fn ensure_rect_capacity(&mut self, verts: usize) {
        // Capacity is stored in "quad slots" (6 verts); round up.
        let needed = (verts + 5) / 6;
        if needed <= self.rect_capacity {
            return;
        }
        let mut cap = self.rect_capacity.max(1);
        while cap < needed {
            cap *= 2;
        }
        crate::App::log(format!(
            "gfx: grow rect buffer {needed} quads / {verts} verts (was {})",
            self.rect_capacity
        ));
        self.rect_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect vertices"),
            size: (cap * 6 * std::mem::size_of::<RectVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.rect_capacity = cap;
    }

    fn ensure_shape_capacity(&mut self, shapes: usize) {
        if shapes <= self.shape_capacity {
            return;
        }
        let mut cap = self.shape_capacity.max(1);
        while cap < shapes {
            cap *= 2;
        }
        crate::App::log(format!(
            "gfx: grow shape buffer {shapes} (was {})",
            self.shape_capacity
        ));
        self.shape_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shape vertices"),
            size: (cap * 6 * std::mem::size_of::<ShapeVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.shape_capacity = cap;
    }

    fn ensure_tex_capacity(&mut self, quads: usize) {
        if quads <= self.tex_capacity {
            return;
        }
        let mut cap = self.tex_capacity.max(1);
        while cap < quads {
            cap *= 2;
        }
        crate::App::log(format!(
            "gfx: grow texture buffer {quads} (was {})",
            self.tex_capacity
        ));
        self.tex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("texture vertices"),
            size: (cap * 6 * std::mem::size_of::<TexVert>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.tex_capacity = cap;
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // `ui_scale` is "surface pixels per logical pixel", so it has to be
        // derived from the *requested* surface size: `fit_dimensions` may clamp
        // it to the device texture limit, and the browser then scales the
        // clamped surface back into the canvas' CSS box. Deriving it from the
        // clamped size shrinks the logical area (e.g. 1365 logical px for a
        // 1400 CSS px canvas), which drifts painting and hit-testing apart.
        let requested = (width.max(1), height.max(1));
        let (clamped_w, clamped_h) = fit_dimensions(requested.0, requested.1, self.max_dim);
        if clamped_w > 0
            && clamped_h > 0
            && (clamped_w != self.config.width || clamped_h != self.config.height)
        {
            self.config.width = clamped_w;
            self.config.height = clamped_h;
            self.surface.configure(&self.device, &self.config);
        }
        self.ui_scale = ui_scale_for(requested.0, requested.1, self.dpr, &self.config);
    }

    /// Configure the surface for `width x height` device pixels while the app
    /// lays out `logical_w x logical_h` CSS pixels. `ui_scale` follows from
    /// both, so a caller that has to render below the device pixel ratio (the
    /// GPU texture limit can be smaller than the window) still gets an exact
    /// logical <-> surface mapping.
    pub fn apply_geometry(
        &mut self,
        dpr: f32,
        width: u32,
        height: u32,
        logical_w: f32,
        logical_h: f32,
    ) {
        self.dpr = if dpr > 0.0 { dpr } else { 1.0 };
        let (width, height) = fit_dimensions(width, height, self.max_dim);
        if width > 0
            && height > 0
            && (width != self.config.width || height != self.config.height)
        {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
        self.ui_scale = (self.config.width as f32 / logical_w.max(1.0))
            .min(self.config.height as f32 / logical_h.max(1.0))
            .max(0.1);
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64, width: u32, height: u32) {
        self.dpr = scale_factor as f32;
        self.resize(width, height);
    }

    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    /// Device pixel ratio this surface was configured for.
    pub fn dpr(&self) -> f32 {
        self.dpr
    }

    /// The device's maximum texture size; a window wider or taller than this
    /// must expose a clamped surface (the browser then scales the canvas).
    pub fn max_dim(&self) -> u32 {
        self.max_dim
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    fn ensure_texture(&mut self, pixmap: &rqt::gui::QPixmap) -> u64 {
        let key = pixmap_key(pixmap);
        if self.texture_cache.contains_key(&key) {
            return key;
        }
        let width = pixmap.width.max(1);
        let height = pixmap.height.max(1);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blit pixmap"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let expected = (width * height * 4) as usize;
        let mut pixels = pixmap.pixels.clone();
        if pixels.len() < expected {
            pixels.resize(expected, 0);
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels[..expected],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&Default::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit bind group"),
            layout: &self.texture_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        self.texture_cache.insert(
            key,
            CachedTexture {
                _texture: texture,
                _view: view,
                bind_group,
            },
        );
        key
    }

    /// Clear the frame, then replay `batches` in painter order.
    pub fn render(&mut self, clear: wgpu::Color, batches: &[PaintBatch]) {
        use wgpu::CurrentSurfaceTexture;

        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture)
            | CurrentSurfaceTexture::Suboptimal(texture) => texture,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };

        let clear = wgpu::Color {
            r: srgb_to_linear(clear.r as f32) as f64,
            g: srgb_to_linear(clear.g as f32) as f64,
            b: srgb_to_linear(clear.b as f32) as f64,
            a: clear.a,
        };

        let surface_w = self.config.width.max(1) as f32;
        let surface_h = self.config.height.max(1) as f32;
        let scale = self.ui_scale;
        self.scratch.clear();
        self.shape_scratch.clear();
        self.tex_scratch.clear();

        enum Built {
            /// Vertex range into `rect_buffer` (rects = 6 verts, tris = 3).
            SolidVerts(u32, u32),
            Shapes(u32, u32),
            Textures(Vec<(u64, u32, u32)>),
            Text(Option<usize>),
        }
        let mut built: Vec<Built> = Vec::with_capacity(batches.len());
        let mut text_count = 0usize;

        for batch in batches {
            match batch {
                PaintBatch::Rects(rects) => {
                    let start = self.scratch.len() as u32;
                    for r in rects {
                        push_rect_quad(
                            &mut self.scratch,
                            r.x,
                            r.y,
                            r.w,
                            r.h,
                            scale,
                            surface_w,
                            surface_h,
                            color_linear(r.color),
                        );
                    }
                    built.push(Built::SolidVerts(start, self.scratch.len() as u32));
                }
                PaintBatch::Triangles(tris) => {
                    let start = self.scratch.len() as u32;
                    for t in tris {
                        push_tri_verts(
                            &mut self.scratch,
                            t,
                            scale,
                            surface_w,
                            surface_h,
                        );
                    }
                    built.push(Built::SolidVerts(start, self.scratch.len() as u32));
                }
                PaintBatch::Shapes(shapes) => {
                    let start = self.shape_scratch.len() as u32;
                    for s in shapes {
                        push_shape_quad(
                            &mut self.shape_scratch,
                            s,
                            scale,
                            surface_w,
                            surface_h,
                        );
                    }
                    built.push(Built::Shapes(start, self.shape_scratch.len() as u32));
                }
                PaintBatch::Textures(textures) => {
                    let mut ranges = Vec::with_capacity(textures.len());
                    for tex in textures {
                        let key = self.ensure_texture(&tex.pixmap);
                        let start = self.tex_scratch.len() as u32;
                        push_tex_quad(
                            &mut self.tex_scratch,
                            tex.x,
                            tex.y,
                            tex.w,
                            tex.h,
                            tex.opacity,
                            scale,
                            surface_w,
                            surface_h,
                        );
                        ranges.push((key, start, self.tex_scratch.len() as u32));
                    }
                    built.push(Built::Textures(ranges));
                }
                PaintBatch::Text(texts) => {
                    if texts.is_empty() {
                        built.push(Built::Text(None));
                    } else {
                        built.push(Built::Text(Some(text_count)));
                        text_count += 1;
                    }
                }
            }
        }

        self.ensure_rect_capacity(self.scratch.len());
        self.ensure_shape_capacity(self.shape_scratch.len() / 6);
        self.ensure_tex_capacity(self.tex_scratch.len() / 6);
        if !self.scratch.is_empty() {
            self.queue
                .write_buffer(&self.rect_buffer, 0, bytemuck::cast_slice(&self.scratch));
        }
        if !self.shape_scratch.is_empty() {
            self.queue.write_buffer(
                &self.shape_buffer,
                0,
                bytemuck::cast_slice(&self.shape_scratch),
            );
        }
        if !self.tex_scratch.is_empty() {
            self.queue.write_buffer(
                &self.tex_buffer,
                0,
                bytemuck::cast_slice(&self.tex_scratch),
            );
        }

        // Shape text runs and upload glyphs into the atlas, one renderer per
        // text batch so the single pass can replay them in display-list order.
        self.text.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
            },
        );
        {
            let state = &mut self.text;
            while state.renderers.len() < text_count {
                let renderer = TextRenderer::new(
                    &mut state.atlas,
                    &self.device,
                    wgpu::MultisampleState::default(),
                    None,
                );
                state.renderers.push(renderer);
            }
        }
        // Quantize metrics so resize float drift does not mint a new glyph
        // set every frame and fill the atlas permanently.
        let font_size = quantize_font_px(FONT_SIZE * scale);
        let line_height = quantize_font_px(LINE_HEIGHT * scale).max(font_size);
        let mut shaped: Vec<(usize, Vec<Buffer>, Vec<&ColoredText>)> =
            Vec::with_capacity(text_count);
        for (batch, slot) in batches.iter().zip(built.iter()) {
            let PaintBatch::Text(texts) = batch else {
                continue;
            };
            let Built::Text(Some(slot)) = slot else {
                continue;
            };
            let mut buffers: Vec<Buffer> = Vec::with_capacity(texts.len());
            for t in texts {
                let mut buffer = Buffer::new(
                    &mut self.text.font_system,
                    Metrics::new(font_size, line_height),
                );
                buffer.set_size(Some(t.wrap_w.max(1.0) * scale), Some(line_height));
                buffer.set_text(
                    &t.text,
                    &Attrs::new().family(Family::Name("DejaVu Sans")),
                    Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(&mut self.text.font_system, false);
                buffers.push(buffer);
            }
            let refs: Vec<&ColoredText> = texts.iter().collect();
            shaped.push((*slot, buffers, refs));
        }

        // AtlasFull must trim+retry: ignoring it leaves chrome without labels
        // until a full reload (seen as flicker during window resize).
        for _attempt in 0..4 {
            let mut atlas_full = false;
            for (slot, buffers, texts) in &shaped {
                let areas: Vec<TextArea> = texts
                    .iter()
                    .zip(buffers.iter())
                    .map(|(t, buffer)| TextArea {
                        buffer,
                        left: t.x * scale,
                        top: t.y * scale,
                        scale: 1.0,
                        bounds: TextBounds {
                            left: (t.clip[0] as f32 * scale) as i32,
                            top: (t.clip[1] as f32 * scale) as i32,
                            right: (t.clip[2] as f32 * scale) as i32,
                            bottom: (t.clip[3] as f32 * scale) as i32,
                        },
                        default_color: Color::rgba(
                            (t.color[0] * 255.0) as u8,
                            (t.color[1] * 255.0) as u8,
                            (t.color[2] * 255.0) as u8,
                            (t.color[3] * 255.0) as u8,
                        ),
                        custom_glyphs: &[],
                    })
                    .collect();
                match self.text.renderers[*slot].prepare(
                    &self.device,
                    &self.queue,
                    &mut self.text.font_system,
                    &mut self.text.atlas,
                    &self.text.viewport,
                    areas,
                    &mut self.text.swash_cache,
                ) {
                    Ok(()) => {}
                    Err(PrepareError::AtlasFull) => {
                        self.text.atlas.trim();
                        atlas_full = true;
                        break;
                    }
                }
            }
            if !atlas_full {
                break;
            }
        }

        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for item in &built {
                match item {
                    Built::SolidVerts(start, end) if end > start => {
                        pass.set_pipeline(&self.rect_pipeline);
                        pass.set_vertex_buffer(0, self.rect_buffer.slice(..));
                        pass.draw(*start..*end, 0..1);
                    }
                    Built::Shapes(start, end) if end > start => {
                        pass.set_pipeline(&self.shape_pipeline);
                        pass.set_vertex_buffer(0, self.shape_buffer.slice(..));
                        pass.draw(*start..*end, 0..1);
                    }
                    Built::Textures(ranges) => {
                        for (key, start, end) in ranges {
                            if end <= start {
                                continue;
                            }
                            let Some(cached) = self.texture_cache.get(key) else {
                                continue;
                            };
                            pass.set_pipeline(&self.texture_pipeline);
                            pass.set_bind_group(0, &cached.bind_group, &[]);
                            pass.set_vertex_buffer(0, self.tex_buffer.slice(..));
                            pass.draw(*start..*end, 0..1);
                        }
                    }
                    Built::Text(Some(slot)) => {
                        let _ = self.text.renderers[*slot].render(
                            &self.text.atlas,
                            &self.text.viewport,
                            &mut pass,
                        );
                    }
                    _ => {}
                }
            }
        }

        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        // Drop unused glyphs so the next resize/frame can allocate again.
        // Without this the atlas fills once and labels stay missing forever.
        self.text.atlas.trim();
    }
}

fn push_rect_quad(
    out: &mut Vec<RectVert>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    scale: f32,
    surface_w: f32,
    surface_h: f32,
    c: [f32; 4],
) {
    let x = (x * scale).round();
    let y = (y * scale).round();
    let w = (w * scale).round().max(1.0);
    let h = (h * scale).round().max(1.0);
    let x0 = x / surface_w * 2.0 - 1.0;
    let x1 = (x + w) / surface_w * 2.0 - 1.0;
    let y0 = 1.0 - y / surface_h * 2.0;
    let y1 = 1.0 - (y + h) / surface_h * 2.0;
    let v = |p: [f32; 2]| RectVert { pos: p, color: c };
    out.push(v([x0, y0]));
    out.push(v([x1, y0]));
    out.push(v([x0, y1]));
    out.push(v([x1, y0]));
    out.push(v([x1, y1]));
    out.push(v([x0, y1]));
}

fn push_tri_verts(
    out: &mut Vec<RectVert>,
    t: &ColoredTriangle,
    scale: f32,
    surface_w: f32,
    surface_h: f32,
) {
    let c = color_linear(t.color);
    let to_ndc = |p: [f32; 2]| {
        let x = (p[0] * scale).round();
        let y = (p[1] * scale).round();
        [
            x / surface_w * 2.0 - 1.0,
            1.0 - y / surface_h * 2.0,
        ]
    };
    out.push(RectVert {
        pos: to_ndc(t.a),
        color: c,
    });
    out.push(RectVert {
        pos: to_ndc(t.b),
        color: c,
    });
    out.push(RectVert {
        pos: to_ndc(t.c),
        color: c,
    });
}

fn push_shape_quad(
    out: &mut Vec<ShapeVert>,
    s: &ShapePrim,
    scale: f32,
    surface_w: f32,
    surface_h: f32,
) {
    let x = (s.x * scale).round();
    let y = (s.y * scale).round();
    let w = (s.w * scale).round().max(1.0);
    let h = (s.h * scale).round().max(1.0);
    let radius = (s.radius * scale).max(0.0);
    let stroke = (s.stroke * scale).max(0.0);
    let half = [w * 0.5, h * 0.5];
    let x0 = x / surface_w * 2.0 - 1.0;
    let x1 = (x + w) / surface_w * 2.0 - 1.0;
    let y0 = 1.0 - y / surface_h * 2.0;
    let y1 = 1.0 - (y + h) / surface_h * 2.0;
    let c0 = color_linear(s.color0);
    // kind 4 packs local endpoints; kind 5 packs dash/gap — scale, don't sRGB.
    let c1 = if s.kind == 4 {
        [
            s.color1[0] * scale,
            s.color1[1] * scale,
            s.color1[2] * scale,
            s.color1[3] * scale,
        ]
    } else if s.kind == 5 {
        [
            s.color1[0] * scale,
            s.color1[1] * scale,
            0.0,
            0.0,
        ]
    } else {
        color_linear(s.color1)
    };
    let params = [radius, stroke, s.kind as f32, 0.0];
    // local coords: origin at rect center, Y down (matches paint space).
    let corners = [
        ([-half[0], -half[1]], [x0, y0]),
        ([half[0], -half[1]], [x1, y0]),
        ([-half[0], half[1]], [x0, y1]),
        ([half[0], -half[1]], [x1, y0]),
        ([half[0], half[1]], [x1, y1]),
        ([-half[0], half[1]], [x0, y1]),
    ];
    for (local, pos) in corners {
        out.push(ShapeVert {
            pos,
            local,
            half_size: half,
            color0: c0,
            color1: c1,
            params,
        });
    }
}

fn push_tex_quad(
    out: &mut Vec<TexVert>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    opacity: f32,
    scale: f32,
    surface_w: f32,
    surface_h: f32,
) {
    let x = (x * scale).round();
    let y = (y * scale).round();
    let w = (w * scale).round().max(1.0);
    let h = (h * scale).round().max(1.0);
    let x0 = x / surface_w * 2.0 - 1.0;
    let x1 = (x + w) / surface_w * 2.0 - 1.0;
    let y0 = 1.0 - y / surface_h * 2.0;
    let y1 = 1.0 - (y + h) / surface_h * 2.0;
    let corners = [
        ([x0, y0], [0.0, 0.0]),
        ([x1, y0], [1.0, 0.0]),
        ([x0, y1], [0.0, 1.0]),
        ([x1, y0], [1.0, 0.0]),
        ([x1, y1], [1.0, 1.0]),
        ([x0, y1], [0.0, 1.0]),
    ];
    for (pos, uv) in corners {
        out.push(TexVert {
            pos,
            uv,
            opacity: opacity.clamp(0.0, 1.0),
            _pad: [0.0; 3],
        });
    }
}

fn quantize_font_px(px: f32) -> f32 {
    ((px * 4.0).round() / 4.0).max(1.0)
}
