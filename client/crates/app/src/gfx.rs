//! wgpu surface on the winit window: configure, clear, present.
//!
//! Stage 3 renderer: a batch of axis-aligned colored rectangles — one
//! draw call per frame, vertices rebuilt from the laid-out widget tree.

use winit::window::Window;

use glyphon::{
    Attrs, Buffer, Cache as GlyphCache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

/// Maximum rectangles per frame (2048 → 12288 vertices).
const MAX_RECTS: usize = 2048;

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

/// One rectangle in pixel coordinates + sRGB-ish color.
#[derive(Debug, Clone, Copy)]
pub struct ColoredRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
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

/// One homogeneous run of the ordered display list. Rects and text are
/// kept in painter order; adjacent commands of the same kind are merged,
/// but a `Rects -> Text -> Rects` sequence is never flattened into two
/// global pools.
#[derive(Debug, Clone)]
pub enum PaintBatch {
    Rects(Vec<ColoredRect>),
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

const VERTEX_ATTRS: [wgpu::VertexAttribute; 2] = [
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

pub struct Gfx {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    max_dim: u32,
    dpr: f32,
    ui_scale: f32,
    rect_pipeline: wgpu::RenderPipeline,
    rect_buffer: wgpu::Buffer,
    scratch: Vec<RectVert>,
    text: TextState,
}

/// `?maxdim=2048` reproduces GPUs (ANGLE/SwiftShader/VMs) whose maximum
/// texture size is smaller than the window, without needing that hardware.
#[cfg(target_arch = "wasm32")]
fn max_dim_override() -> Option<u32> {
    let window = web_sys::window()?;
    let value = js_sys::Reflect::get(&window, &"__epMaxDim".into()).ok()?;
    let value = value.as_f64()?;
    (value >= 1.0).then_some(value as u32)
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

        // The WebGL2 backend reports lower limits than WebGPU does.
        let required_limits = if info.backend == wgpu::Backend::Gl {
            wgpu::Limits::downlevel_webgl2_defaults()
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
        #[cfg(target_arch = "wasm32")]
        let max_dim = max_dim_override().unwrap_or(device.limits().max_texture_dimension_2d);
        #[cfg(not(target_arch = "wasm32"))]
        let max_dim = device.limits().max_texture_dimension_2d;
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_WGSL.into()),
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVert>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRS,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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

        let rect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect vertices"),
            size: (MAX_RECTS * 6 * std::mem::size_of::<RectVert>()) as u64,
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
            rect_buffer,
            scratch: Vec::with_capacity(MAX_RECTS * 6),
            text,
        })
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

    /// Clear the frame, then replay `batches` in painter order. Rect runs
    /// share one vertex buffer; each text run is prepared into its own
    /// glyphon renderer so text can appear between rectangles.
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

        // Pixel rects → NDC triangles. Each rect batch keeps a vertex range
        // into the single shared buffer; batches are replayed in order.
        let surface_w = self.config.width.max(1) as f32;
        let surface_h = self.config.height.max(1) as f32;
        let scale = self.ui_scale;
        self.scratch.clear();
        let mut ranges = Vec::with_capacity(batches.len());
        let mut text_slots = Vec::with_capacity(batches.len());
        let mut text_count = 0usize;
        for batch in batches {
            match batch {
                PaintBatch::Rects(rects) => {
                    let start = self.scratch.len() as u32;
                    for r in rects
                        .iter()
                        .take(MAX_RECTS.saturating_sub(start as usize / 6))
                    {
                        let x = (r.x * scale).round();
                        let y = (r.y * scale).round();
                        let w = (r.w * scale).round().max(1.0);
                        let h = (r.h * scale).round().max(1.0);
                        let x0 = x / surface_w * 2.0 - 1.0;
                        let x1 = (x + w) / surface_w * 2.0 - 1.0;
                        let y0 = 1.0 - y / surface_h * 2.0;
                        let y1 = 1.0 - (y + h) / surface_h * 2.0;
                        let c = [
                            srgb_to_linear(r.color[0]),
                            srgb_to_linear(r.color[1]),
                            srgb_to_linear(r.color[2]),
                            r.color[3],
                        ];
                        let v = |p: [f32; 2]| RectVert { pos: p, color: c };
                        // Two triangles per rect.
                        self.scratch.push(v([x0, y0]));
                        self.scratch.push(v([x1, y0]));
                        self.scratch.push(v([x0, y1]));
                        self.scratch.push(v([x1, y0]));
                        self.scratch.push(v([x1, y1]));
                        self.scratch.push(v([x0, y1]));
                    }
                    ranges.push(Some((start, self.scratch.len() as u32)));
                    text_slots.push(None);
                }
                PaintBatch::Text(texts) => {
                    ranges.push(None);
                    if texts.is_empty() {
                        text_slots.push(None);
                    } else {
                        text_slots.push(Some(text_count));
                        text_count += 1;
                    }
                }
            }
        }
        if !self.scratch.is_empty() {
            self.queue
                .write_buffer(&self.rect_buffer, 0, bytemuck::cast_slice(&self.scratch));
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
        let font_size = FONT_SIZE * scale;
        let line_height = LINE_HEIGHT * scale;
        for (batch, slot) in batches.iter().zip(&text_slots) {
            let PaintBatch::Text(texts) = batch else {
                continue;
            };
            let Some(slot) = slot else {
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
            let areas: Vec<TextArea> = texts
                .iter()
                .zip(&buffers)
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
            // Atlas-full errors are transient: the run is retried next frame.
            let _ = self.text.renderers[*slot].prepare(
                &self.device,
                &self.queue,
                &mut self.text.font_system,
                &mut self.text.atlas,
                &self.text.viewport,
                areas,
                &mut self.text.swash_cache,
            );
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
            for ((batch, range), slot) in batches.iter().zip(&ranges).zip(&text_slots) {
                match batch {
                    PaintBatch::Rects(_) => {
                        if let Some((start, end)) = range {
                            if end > start {
                                pass.set_pipeline(&self.rect_pipeline);
                                pass.set_vertex_buffer(0, self.rect_buffer.slice(..));
                                pass.draw(*start..*end, 0..1);
                            }
                        }
                    }
                    PaintBatch::Text(_) => {
                        if let Some(slot) = slot {
                            let _ = self.text.renderers[*slot].render(
                                &self.text.atlas,
                                &self.text.viewport,
                                &mut pass,
                            );
                        }
                    }
                }
            }
        }

        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
    }
}
