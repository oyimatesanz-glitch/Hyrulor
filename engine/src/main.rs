//! Hyrulor Engine — motor gráfico nativo (winit + wgpu + egui).
//!
//! Renderiza un orbe holográfico cuyo color/pulso codifican el estado del
//! sistema y una interfaz de chat que habla con el daemon Hyrulor por socket
//! Unix (JSON-Lines). La autorización humana de acciones ofensivas se muestra
//! como formulario nativo dentro de la ventana.

mod daemon;
mod orb;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

use glam::{Mat4, Vec3};
use orb::{create_sphere, create_torus, OrbState, Vertex};
use serde_json::Value;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// Paleta del diseño Hyrulor (paridad con `designs/index.html`).
const GOLD: egui::Color32 = egui::Color32::from_rgb(212, 175, 55);
/// Borde sutil entre secciones (paridad con `--border-subtle`).
const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(40, 40, 46);
/// Grosor de los bordes de la UI.
const STROKE: f32 = 1.0;

/// Evento que un hilo trabajador devuelve al bucle de render.
enum Event {
    Status {
        online: bool,
        mode: String,
        detail: Option<String>,
    },
    Reply(ChatOutcome),
}

/// Resultado de una petición de chat o de autorización.
enum ChatOutcome {
    Reply(daemon::Reply),
    Unreachable(String),
}

/// Autorización pendiente (human-in-the-loop) mostrada en la UI.
struct AuthState {
    mode: String,
    request_id: String,
    form: Value,
    values: HashMap<String, String>,
}

struct State {
    // wgpu
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    scene_color: wgpu::Texture,
    scene_color_view: wgpu::TextureView,
    #[allow(dead_code)]
    bloom_color: wgpu::Texture,
    bloom_color_view: wgpu::TextureView,
    fullscreen_vertex_buffer: wgpu::Buffer,
    post_bind_group: wgpu::BindGroup,
    bloom_bind_group: wgpu::BindGroup,
    bloom_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,

    sphere_vertex_buffer: wgpu::Buffer,
    sphere_index_buffer: wgpu::Buffer,
    sphere_num_indices: u32,
    ring_vertex_buffer: wgpu::Buffer,
    ring_index_buffer: wgpu::Buffer,
    ring_num_indices: u32,
    sphere_uniform: wgpu::Buffer,
    sphere_bind_group: wgpu::BindGroup,
    ring_uniform: wgpu::Buffer,
    ring_bind_group: wgpu::BindGroup,
    start_time: Instant,

    // egui
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    // chat
    chat_input: String,
    chat_history: Vec<String>,

    // daemon
    socket: PathBuf,
    mode: String,
    orb_state: OrbState,
    auth: Option<AuthState>,
    last_status_check: Instant,
    status_inflight: bool,

    // workers
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
}

/// Consulta salud y modo del daemon en un hilo, sin bloquear el render.
fn spawn_status_check(socket: PathBuf, sender: mpsc::Sender<Event>) {
    thread::spawn(move || {
        let online = daemon::health(&socket).is_ok();
        let mode = daemon::get_mode(&socket)
            .ok()
            .and_then(|reply| match reply {
                daemon::Reply::Ok(data) => {
                    data.get("mode").and_then(Value::as_str).map(str::to_string)
                }
                _ => None,
            })
            .unwrap_or_else(|| "simple".to_string());
        let detail = if online {
            None
        } else {
            Some("daemon no accesible; el orbe queda en modo sin conexión.".to_string())
        };
        let _ = sender.send(Event::Status {
            online,
            mode,
            detail,
        });
    });
}

/// Envía una pregunta al daemon en un hilo.
fn send_chat(query: String, socket: PathBuf, sender: mpsc::Sender<Event>) {
    thread::spawn(move || {
        let outcome = match daemon::chat(&socket, &query) {
            Ok(reply) => ChatOutcome::Reply(reply),
            Err(error) => ChatOutcome::Unreachable(error),
        };
        let _ = sender.send(Event::Reply(outcome));
    });
}

/// Aprueba o cancela una autorización pendiente en un hilo.
fn send_authorize(auth: AuthState, approved: bool, socket: PathBuf, sender: mpsc::Sender<Event>) {
    thread::spawn(move || {
        let form = if approved && auth.mode == "secure" {
            Some(Value::Object(
                auth.values
                    .into_iter()
                    .map(|(key, value)| (key, Value::String(value)))
                    .collect(),
            ))
        } else {
            None
        };
        let outcome = match daemon::authorize(&socket, &auth.request_id, approved, form) {
            Ok(reply) => ChatOutcome::Reply(reply),
            Err(error) => ChatOutcome::Unreachable(error),
        };
        let _ = sender.send(Event::Reply(outcome));
    });
}

/// Aplica un evento del daemon al estado del render.
fn apply_event(state: &mut State, event: Event) {
    match event {
        Event::Status {
            online,
            mode,
            detail,
        } => {
            state.status_inflight = false;
            state.last_status_check = Instant::now();
            let was_offline = state.orb_state == OrbState::Offline;
            state.mode = mode;
            state.orb_state = if online {
                OrbState::Idle
            } else {
                OrbState::Offline
            };
            if online && was_offline {
                eprintln!("[hyrulor-engine] daemon conectado (modo {})", state.mode);
                state
                    .chat_history
                    .push("Hyrulor: daemon conectado.".to_string());
            }
            // Avisar solo al pasar a sin conexión, no en cada sonda.
            if let Some(detail) = detail {
                if !was_offline {
                    eprintln!("[hyrulor-engine] daemon no accesible: {detail}");
                    state.chat_history.push(format!("Hyrulor: {detail}"));
                }
            }
        }
        Event::Reply(outcome) => match outcome {
            ChatOutcome::Unreachable(error) => {
                state.orb_state = OrbState::Offline;
                state
                    .chat_history
                    .push(format!("Hyrulor: sin conexión con el daemon: {error}"));
                state.chat_history.push(
                    "Arranca el daemon (./hyrulor.sh start) y verifica el socket.".to_string(),
                );
            }
            ChatOutcome::Reply(reply) => match reply {
                daemon::Reply::Ok(data) => {
                    let answer = data
                        .get("answer")
                        .and_then(Value::as_str)
                        .unwrap_or("(respuesta vacía)");
                    state.chat_history.push(format!("Hyrulor: {answer}"));
                    state.orb_state = OrbState::Idle;
                }
                daemon::Reply::Error(message) => {
                    state.chat_history.push(format!("Hyrulor: {message}"));
                    state.orb_state = OrbState::Error;
                }
                daemon::Reply::RequiresAuth {
                    mode,
                    request_id,
                    form,
                } => {
                    state.auth = Some(AuthState {
                        mode,
                        request_id,
                        form,
                        values: HashMap::new(),
                    });
                    state.orb_state = OrbState::Blocked;
                }
            },
        },
    }
}

/// Rellena el uniforme plano del shader.
fn fill_uniform(uniform: &mut [f32; 32], mvp: &Mat4, orb_state: OrbState, elapsed: f32) {
    uniform[0..16].copy_from_slice(&mvp.to_cols_array());
    uniform[16] = elapsed;
    let color = orb_state.color();
    uniform[17] = color[0];
    uniform[18] = color[1];
    uniform[19] = color[2];
    // Cámara fija en el eje +Z (misma que `look_at_rh`).
    uniform[20] = 0.0;
    uniform[21] = 0.0;
    uniform[22] = 5.0;
    uniform[23] = orb_state.pulse_speed();
    uniform[24] = orb_state.fresnel_power();
    uniform[25] = orb_state.brightness();
}

/// Color de la insignia de estado para egui.
fn badge_color(state: OrbState) -> egui::Color32 {
    match state {
        OrbState::Idle => egui::Color32::from_rgb(70, 200, 130),
        OrbState::Thinking => egui::Color32::from_rgb(212, 175, 55),
        OrbState::Blocked => egui::Color32::from_rgb(220, 130, 70),
        OrbState::Error => egui::Color32::from_rgb(210, 70, 55),
        OrbState::Offline => egui::Color32::from_rgb(120, 120, 125),
    }
}

struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Hyrulor Engine v0.9 — Orbe holográfico + chat nativo")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 760.0));
        let window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(window);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orb-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/orb.wgsl").into()),
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("orb-post-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/post.wgsl").into()),
        });

        // 32 f32 = 128 bytes por objeto. Un buffer distinto para la esfera y
        // otro para el anillo: así cada `write_buffer` escribe un único buffer
        // una sola vez por frame (evita el doble write sobre el mismo buffer
        // dentro de un render pass).
        let uniform_size = (32 * std::mem::size_of::<f32>()) as u64;
        let sphere_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sphere-uniform"),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ring_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ring-uniform"),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let sphere_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sphere-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sphere_uniform.as_entire_binding(),
            }],
        });
        let ring_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ring-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ring_uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orb-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[Vertex::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let post_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&post_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let scene_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene-color"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let scene_color_view = scene_color.create_view(&wgpu::TextureViewDescriptor::default());
        let bloom_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom-color"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let bloom_color_view = bloom_color.create_view(&wgpu::TextureViewDescriptor::default());

        let fullscreen_verts: Vec<f32> = vec![-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];
        let fullscreen_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("fullscreen-vertices"),
                contents: bytemuck::cast_slice(&fullscreen_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post-bind-group"),
            layout: &post_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&bloom_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Bind group exclusivo del pase de bloom. El shader de bloom solo
        // usa la escena y el sampler, pero el layout tiene 3 slots: se enlaza
        // la escena también en el slot 1 para no referenciar `bloom_color_view`
        // mientras esa misma textura es el color target del pass (uso
        // exclusivo COLOR_TARGET incompatible con RESOURCE).
        let bloom_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom-bind-group"),
            layout: &post_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&scene_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let fullscreen_vertex_state = wgpu::VertexState {
            module: &post_shader,
            entry_point: "vs_main",
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                }],
            }],
        };

        let bloom_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orb-bloom-pipeline"),
            layout: Some(&post_pipeline_layout),
            vertex: fullscreen_vertex_state.clone(),
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: "fs_bloom",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("orb-composite-pipeline"),
            layout: Some(&post_pipeline_layout),
            vertex: fullscreen_vertex_state.clone(),
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: "fs_composite",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let (sphere_vertices, sphere_indices) = create_sphere(1.5, 32, 16);
        let sphere_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere-vertices"),
            contents: bytemuck::cast_slice(&sphere_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sphere_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sphere-indices"),
            contents: bytemuck::cast_slice(&sphere_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (ring_vertices, ring_indices) = create_torus(2.2, 0.05, 16, 64);
        let ring_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ring-vertices"),
            contents: bytemuck::cast_slice(&ring_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ring_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ring-indices"),
            contents: bytemuck::cast_slice(&ring_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let egui_ctx = egui::Context::default();
        let mut style = (*egui_ctx.style()).clone();
        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.window_rounding = egui::Rounding::same(24.0);
        visuals.window_fill = egui::Color32::from_rgb(8, 8, 9);
        visuals.panel_fill = egui::Color32::from_rgba_premultiplied(10, 10, 10, 235);
        visuals.extreme_bg_color = egui::Color32::from_rgb(8, 8, 9);
        visuals.hyperlink_color = GOLD;
        visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(212, 175, 55, 90);

        // Radio mayor para que el campo de texto se vea "pill"; los botones
        // dorados fijan su propio redondeado (circular) con `.rounding(..)`.
        let widget_rounding = egui::Rounding::same(14.0);
        let light_fg = egui::Color32::from_rgb(235, 232, 225);
        visuals.widgets.noninteractive.bg_fill =
            egui::Color32::from_rgba_premultiplied(15, 15, 17, 230);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(STROKE, BORDER_SUBTLE);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(STROKE, light_fg);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(24, 24, 27, 235);
        visuals.widgets.inactive.rounding = widget_rounding;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(STROKE, BORDER_SUBTLE);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(STROKE, light_fg);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(38, 38, 42, 240);
        visuals.widgets.hovered.rounding = widget_rounding;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(STROKE, GOLD);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(STROKE, light_fg);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgba_premultiplied(30, 30, 34, 240);
        visuals.widgets.active.rounding = widget_rounding;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(STROKE, GOLD);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(STROKE, light_fg);

        // El handle del scrollbar se pinta con los visuales de widget (dorado
        // al pasar, oro claro al arrastrar, oscuro en reposo).
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.scroll.handle_min_length = 24.0;

        style.spacing.item_spacing = egui::vec2(10.0, 14.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        egui_ctx.set_style(style);

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1, true);

        let (sender, receiver) = mpsc::channel::<Event>();
        let socket = daemon::socket_path();

        self.state = Some(State {
            surface,
            device,
            queue,
            config,
            window,
            render_pipeline,
            scene_color,
            scene_color_view,
            bloom_color,
            bloom_color_view,
            fullscreen_vertex_buffer,
            post_bind_group,
            bloom_bind_group,
            bloom_pipeline,
            composite_pipeline,
            sphere_vertex_buffer,
            sphere_index_buffer,
            sphere_num_indices: sphere_indices.len() as u32,
            ring_vertex_buffer,
            ring_index_buffer,
            ring_num_indices: ring_indices.len() as u32,
            sphere_uniform,
            sphere_bind_group,
            ring_uniform,
            ring_bind_group,
            start_time: Instant::now(),
            egui_ctx,
            egui_state,
            egui_renderer,
            chat_input: String::new(),
            chat_history: vec!["Hyrulor Engine v0.9 — orbe holográfico y chat nativo.".to_string()],
            socket: socket.clone(),
            mode: "simple".to_string(),
            orb_state: OrbState::Offline,
            auth: None,
            last_status_check: Instant::now(),
            status_inflight: true,
            sender,
            receiver,
        });

        spawn_status_check(socket, self.state.as_ref().unwrap().sender.clone());

        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        let response = state.egui_state.on_window_event(&state.window, &event);
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    state.config.width = new_size.width;
                    state.config.height = new_size.height;
                    state.surface.configure(&state.device, &state.config);
                }
            }
            WindowEvent::RedrawRequested => {
                // 1. Drenar eventos de los hilos trabajadores.
                let mut events = Vec::new();
                while let Ok(event) = state.receiver.try_recv() {
                    events.push(event);
                }
                for event in events {
                    apply_event(state, event);
                }

                // Sonda periódica: detecta que el daemon arranca o se cae
                // aunque el usuario no envíe ningún mensaje.
                if !state.status_inflight
                    && state.last_status_check.elapsed() >= Duration::from_secs(5)
                {
                    state.status_inflight = true;
                    spawn_status_check(state.socket.clone(), state.sender.clone());
                }

                let elapsed = state.start_time.elapsed().as_secs_f32();
                let aspect = state.config.width as f32 / state.config.height as f32;
                let proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 100.0);
                let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);

                let model_sphere =
                    Mat4::from_rotation_x(elapsed * 0.5) * Mat4::from_rotation_y(elapsed * 0.8);
                let mvp_sphere = proj * view * model_sphere;
                let model_ring =
                    Mat4::from_rotation_z(elapsed * 1.5) * Mat4::from_rotation_x(elapsed * 0.3);
                let mvp_ring = proj * view * model_ring;

                let mut uniform_sphere = [0.0f32; 32];
                fill_uniform(&mut uniform_sphere, &mvp_sphere, state.orb_state, elapsed);
                let mut uniform_ring = [0.0f32; 32];
                fill_uniform(&mut uniform_ring, &mvp_ring, state.orb_state, elapsed);

                let raw_input = state.egui_state.take_egui_input(&state.window);
                let egui_output = state.egui_ctx.run(raw_input, |ctx| {
                    let gold = GOLD;
                    let border = egui::Color32::from_rgb(40, 40, 46);
                    let frame = egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(8, 8, 9, 235))
                        .rounding(egui::Rounding::same(24.0))
                        .stroke(egui::Stroke::new(STROKE, border))
                        .inner_margin(egui::Margin::same(24.0));

                    let mut auth_action: Option<bool> = None;

                    egui::Window::new("Hyrulor")
                        .title_bar(false)
                        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-30.0, -30.0))
                        .resizable(false)
                        .frame(frame)
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                // Avatar circular dorado (paridad con `.chat-avatar`).
                                let avatar_size = 36.0;
                                let (avatar_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(avatar_size, avatar_size),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter_at(avatar_rect);
                                painter.circle_filled(
                                    avatar_rect.center(),
                                    avatar_size / 2.0,
                                    gold,
                                );
                                painter.text(
                                    avatar_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "H",
                                    egui::FontId::proportional(15.0),
                                    egui::Color32::BLACK,
                                );
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new("HYRULOR")
                                        .color(gold)
                                        .size(14.0)
                                        .strong(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let mode_label = if state.mode == "secure" {
                                            "🔐 SEGURO"
                                        } else {
                                            "🔓 SIMPLE"
                                        };
                                        ui.label(
                                            egui::RichText::new(mode_label)
                                                .color(gold)
                                                .monospace()
                                                .size(10.5),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "● {}",
                                                state.orb_state.label()
                                            ))
                                            .color(badge_color(state.orb_state))
                                            .monospace()
                                            .size(10.5),
                                        );
                                    },
                                );
                            });
                            ui.separator();

                            // Cuerpo del chat: burbujas por rol, alineadas al
                            // diseño (usuario a la derecha en oro, asistente
                            // a la izquierda en cristal oscuro).
                            egui::ScrollArea::vertical()
                                .id_salt("chat-body")
                                .max_height(300.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.set_min_width(360.0);
                                    for msg in &state.chat_history {
                                        let is_user = msg.starts_with("Tú:");
                                        let display = msg
                                            .strip_prefix("Tú: ")
                                            .or_else(|| msg.strip_prefix("Hyrulor: "))
                                            .unwrap_or(msg);
                                        let bubble_fill = if is_user {
                                            egui::Color32::from_rgba_premultiplied(212, 175, 55, 22)
                                        } else {
                                            egui::Color32::from_rgba_premultiplied(24, 24, 27, 210)
                                        };
                                        let bubble_border = if is_user {
                                            egui::Color32::from_rgb(212, 175, 55)
                                        } else {
                                            border
                                        };
                                        let bubble_rounding = if is_user {
                                            egui::Rounding {
                                                nw: 18.0,
                                                ne: 4.0,
                                                sw: 18.0,
                                                se: 18.0,
                                            }
                                        } else {
                                            egui::Rounding {
                                                nw: 4.0,
                                                ne: 18.0,
                                                sw: 18.0,
                                                se: 18.0,
                                            }
                                        };
                                        let bubble = egui::Frame::none()
                                            .fill(bubble_fill)
                                            .rounding(bubble_rounding)
                                            .stroke(egui::Stroke::new(STROKE, bubble_border))
                                            .inner_margin(egui::Margin {
                                                left: 16.0,
                                                right: 16.0,
                                                top: 11.0,
                                                bottom: 11.0,
                                            });
                                        ui.with_layout(
                                            if is_user {
                                                egui::Layout::right_to_left(egui::Align::TOP)
                                            } else {
                                                egui::Layout::left_to_right(egui::Align::TOP)
                                            },
                                            |ui| {
                                                bubble.show(ui, |ui| {
                                                    ui.set_max_width(340.0);
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(
                                                                display.to_string(),
                                                            )
                                                            .color(egui::Color32::from_rgb(
                                                                238, 236, 230,
                                                            )),
                                                        )
                                                        .wrap_mode(egui::TextWrapMode::Extend),
                                                    );
                                                });
                                            },
                                        );
                                        ui.add_space(14.0);
                                    }
                                });

                            ui.add_space(10.0);

                            // Panel de autorización humana (modo simple o seguro).
                            if let Some(auth) = &mut state.auth {
                                ui.separator();
                                ui.label(
                                    egui::RichText::new("⚠️ ACCIÓN PENDIENTE DE AUTORIZACIÓN")
                                        .color(egui::Color32::from_rgb(220, 130, 70))
                                        .strong(),
                                );

                                if let Some(question) =
                                    auth.form.get("question").and_then(Value::as_str)
                                {
                                    ui.label(question);
                                }
                                if let Some(title) = auth.form.get("title").and_then(Value::as_str)
                                {
                                    if title != "¿Seguro que quieres hacer esto?" {
                                        ui.label(title);
                                    }
                                }
                                if let Some(action) = auth.form.get("proposed_action") {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Acción propuesta: {}",
                                            action
                                        ))
                                        .monospace()
                                        .small(),
                                    );
                                }

                                if auth.mode == "secure" {
                                    if let Some(code) =
                                        auth.form.get("authorization_code").and_then(Value::as_str)
                                    {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "CÓDIGO DE AUTORIZACIÓN: {code}"
                                            ))
                                            .color(gold)
                                            .monospace(),
                                        );
                                    }
                                    if let Some(fields) =
                                        auth.form.get("fields").and_then(Value::as_array)
                                    {
                                        for field in fields {
                                            let name = field
                                                .get("name")
                                                .and_then(Value::as_str)
                                                .unwrap_or("campo")
                                                .to_string();
                                            let label = field
                                                .get("label")
                                                .and_then(Value::as_str)
                                                .unwrap_or(&name)
                                                .to_string();
                                            let required = field
                                                .get("required")
                                                .and_then(Value::as_bool)
                                                .unwrap_or(true);
                                            let value =
                                                auth.values.entry(name.clone()).or_default();
                                            ui.horizontal(|ui| {
                                                ui.label(format!(
                                                    "{label}{}",
                                                    if required { " *" } else { "" }
                                                ));
                                                ui.add(
                                                    egui::TextEdit::singleline(value)
                                                        .desired_width(240.0),
                                                );
                                            });
                                        }
                                    }
                                }

                                ui.horizontal(|ui| {
                                    if ui
                                        .button(
                                            egui::RichText::new("Autorizar")
                                                .color(egui::Color32::BLACK),
                                        )
                                        .clicked()
                                    {
                                        auth_action = Some(true);
                                    }
                                    if ui.button("Cancelar").clicked() {
                                        auth_action = Some(false);
                                    }
                                });
                            }

                            ui.add_space(6.0);
                            ui.separator();

                            let thinking = state.orb_state == OrbState::Thinking;
                            ui.horizontal(|ui| {
                                // Campo "pill": usa el redondeado de widget (14)
                                // y el fondo oscuro de `widgets.inactive`.
                                let text_edit = egui::TextEdit::singleline(&mut state.chat_input)
                                    .hint_text("Escribe un mensaje…")
                                    .text_color(egui::Color32::from_rgb(238, 236, 230))
                                    .frame(true)
                                    .desired_width(f32::INFINITY);
                                ui.add(text_edit);
                                ui.add_space(10.0);

                                let send_button = ui.add_enabled(
                                    !thinking && !state.chat_input.trim().is_empty(),
                                    egui::Button::new(
                                        egui::RichText::new("➤")
                                            .color(egui::Color32::BLACK)
                                            .size(18.0),
                                    )
                                    .fill(gold)
                                    .rounding(egui::Rounding::same(23.0))
                                    .frame(true)
                                    .min_size(egui::vec2(46.0, 46.0)),
                                );
                                if send_button.clicked() {
                                    let user_msg = state.chat_input.trim().to_string();
                                    state.chat_history.push(format!("Tú: {user_msg}"));
                                    state.orb_state = OrbState::Thinking;
                                    send_chat(user_msg, state.socket.clone(), state.sender.clone());
                                    state.chat_input.clear();
                                }
                            });
                        });

                    // Procesar la decisión de autorización fuera del préstamo
                    // de `state.auth`.
                    if let Some(approved) = auth_action {
                        if let Some(auth) = state.auth.take() {
                            state.orb_state = OrbState::Thinking;
                            send_authorize(
                                auth,
                                approved,
                                state.socket.clone(),
                                state.sender.clone(),
                            );
                        }
                    }
                });

                state
                    .egui_state
                    .handle_platform_output(&state.window, egui_output.platform_output);

                for (id, image_delta) in &egui_output.textures_delta.set {
                    state.egui_renderer.update_texture(
                        &state.device,
                        &state.queue,
                        *id,
                        image_delta,
                    );
                }
                for id in &egui_output.textures_delta.free {
                    state.egui_renderer.free_texture(id);
                }

                let paint_jobs = state
                    .egui_ctx
                    .tessellate(egui_output.shapes, egui_output.pixels_per_point);
                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [state.config.width, state.config.height],
                    pixels_per_point: egui_output.pixels_per_point,
                };

                let mut encoder = state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                let user_cmd_bufs = state.egui_renderer.update_buffers(
                    &state.device,
                    &state.queue,
                    &mut encoder,
                    &paint_jobs,
                    &screen_descriptor,
                );

                let frame = match state.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        // La superficie cambió (redimensionado, monitor, etc.):
                        // se reconfigura y se reintenta en el siguiente frame.
                        state.surface.configure(&state.device, &state.config);
                        state.window.request_redraw();
                        return;
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        state.window.request_redraw();
                        return;
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        event_loop.exit();
                        return;
                    }
                };
                let view_tex = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Escribir cada uniforme una sola vez, antes de grabar el pass
                // (nunca dos writes sobre el mismo buffer dentro del pass).
                state.queue.write_buffer(
                    &state.sphere_uniform,
                    0,
                    bytemuck::cast_slice(&uniform_sphere),
                );
                state.queue.write_buffer(
                    &state.ring_uniform,
                    0,
                    bytemuck::cast_slice(&uniform_ring),
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("scene-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &state.scene_color_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.02,
                                    g: 0.02,
                                    b: 0.02,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(&state.render_pipeline);

                    render_pass.set_bind_group(0, &state.sphere_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, state.sphere_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        state.sphere_index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..state.sphere_num_indices, 0, 0..1);

                    render_pass.set_bind_group(0, &state.ring_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, state.ring_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        state.ring_index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..state.ring_num_indices, 0, 0..1);
                }

                // Fase 8: Bloom — scene_color -> bloom_color.
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bloom-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &state.bloom_color_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::default()),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(&state.bloom_pipeline);
                    render_pass.set_bind_group(0, &state.bloom_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, state.fullscreen_vertex_buffer.slice(..));
                    render_pass.draw(0..3u32, 0..1);
                }

                // Fase 8: Composite — scene_color + bloom -> pantalla.
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("composite-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view_tex,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(&state.composite_pipeline);
                    render_pass.set_bind_group(0, &state.post_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, state.fullscreen_vertex_buffer.slice(..));
                    render_pass.draw(0..3u32, 0..1);
                }

                {
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui-render-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view_tex,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    state.egui_renderer.render(
                        &mut render_pass.forget_lifetime(),
                        &paint_jobs,
                        &screen_descriptor,
                    );
                }

                state.queue.submit(
                    user_cmd_bufs
                        .into_iter()
                        .chain(std::iter::once(encoder.finish())),
                );
                frame.present();
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
