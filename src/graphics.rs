use std::{borrow::Cow, sync::Arc, time::Instant};

use crate::app::AppState;
use nanorand::{Rng, WyRand};
use wgpu::{util::DeviceExt, TextureView};
use winit::window::Window;

const CELLS_PER_GROUP: u32 = 50; // lower is better perfomance, but too low is complete madness

pub struct WgpuContext {
    /// winnit window representation
    pub window: Arc<Window>,

    /// where the stuff will be drawn
    pub surface: wgpu::Surface<'static>,
    /// parameters (texture,...) of the surface
    pub surface_config: wgpu::SurfaceConfiguration,

    pub device: wgpu::Device,
    queue: wgpu::Queue,

    // bind groups
    cells_compute_bind_groups: Vec<wgpu::BindGroup>,
    draw_bind_groups: wgpu::BindGroup,

    // buffers
    cells_buffers: Vec<wgpu::Buffer>, // src and dst so only 2
    vertices_buffer: wgpu::Buffer,    // contain square vertices

    // pipelines
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,

    // extra
    work_group_count: u32,
    pub frame_num: usize,
    pub since_last_frame: Instant,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>, state: &AppState) -> WgpuContext {
        // instantiate basic wgpu modules
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // check that compute shader are supported
        let downlevel_capabilities = adapter.get_downlevel_capabilities();
        assert!(
            downlevel_capabilities
                .flags
                .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS),
            "Adapter does not support the downlevel capabilities required to run a compute shader"
        );

        // request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device descriptor"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();

        // configure the surface now that we have a device
        let mut config = surface
            .get_default_config(&adapter, state.width, state.height)
            .expect("Surface isn't supported by the adapter.");
        let surface_caps = surface.get_capabilities(&adapter);
        let is_srgb = surface_caps.formats.iter().any(|f| f.is_srgb());
        if is_srgb {
            // Not all platforms (WebGPU) support sRGB swapchains, so we need to use view formats
            let view_format = config.format.add_srgb_suffix();
            config.view_formats.push(view_format);
        } else {
            // All platforms support non-sRGB swapchains, so we can just use the format directly.
            let format = config.format.remove_srgb_suffix();
            config.format = format;
            config.view_formats.push(format);
        };
        surface.configure(&device, &config);

        // fetch shaders

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shaders/compute.wgsl"))),
        });
        let draw_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Draw shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shaders/draw.wgsl"))),
        });

        // buffer for simulation parameters uniform

        let sim_param_data = vec![
            state.width,
            state.height,
            state.cell_dimension,
            state.cell_number_x,
            state.cell_number_y,
            state.total_cell_number,
            state.color_number,
        ];
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation Parameter Buffer"),
            contents: bytemuck::cast_slice(&sim_param_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // create compute bind layout group and compute pipeline layout

        /// represent the size in bytes taken by an 'u32' in memory, which is 4
        const SIZE_OF_U32: u64 = std::mem::size_of::<u32>() as u64;
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_param_data.len() as u64) * SIZE_OF_U32,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                // shader variable 'cellSrc' is of type array<u32> of len total_cell_number
                                (state.total_cell_number as u64) * SIZE_OF_U32,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                // shader variable 'cellDst' is of type array<u32> of len total_cell_number
                                (state.total_cell_number as u64) * SIZE_OF_U32,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("Compute bind groups"),
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute pipeline layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        // create render pipeline with bind groups
        const SIZE_OF_F32: u64 = std::mem::size_of::<f32>() as u64;
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, // only vertex need an access to global parameters
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (sim_param_data.len() as u64) * SIZE_OF_U32,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, // only the fragment need to have access to the cell color
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                // shader variable 'colormap' is of type array<[u32; u32; u32]> of len NUM_COLOR
                                (state.color_number as u64) * 3 * SIZE_OF_F32,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("Render bind groups"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main_vs",
                buffers: &[
                    // 'cell_index' and 'color_index' variable from the draw shader, which is updated at each new cell (instance)
                    wgpu::VertexBufferLayout {
                        #[allow(clippy::identity_op)]
                        array_stride: 1 * SIZE_OF_U32, // we store only 1 u32
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Uint32],
                    },
                    // 'vspos' variable from draw shader that will take data from the vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: 2 * SIZE_OF_U32, // we store through the array 2 u32
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Float32x2],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main_fs",
                targets: &[Some(config.view_formats[0].into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // create compute pipeline

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // buffer for the four 2d square vertices of each instance

        /// 6 points in ccw order: (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)  representing a single square taking all screen
        const SQUARE_VERTEX: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];
        let min_number_of_cell = state.cell_number_x.min(state.cell_number_y);

        // square scaled down by a factor of the minimum
        let vertex_buffer_data = SQUARE_VERTEX.map(|coord| coord / min_number_of_cell as f32);
        let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&vertex_buffer_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // buffer for all cell color

        let mut rng = WyRand::new();
        let initial_cell_data = (0..state.total_cell_number)
            .map(|_| rng.generate_range(0_u32..state.color_number))
            .collect::<Vec<u32>>();

        // creates two buffers of cell data each of size total_cell_number
        // the two buffers alternate as dst and src for each frame

        let mut cells_buffers = Vec::<wgpu::Buffer>::new();
        for i in 0..2 {
            cells_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Cell Buffer {i}")),
                    contents: bytemuck::cast_slice(&initial_cell_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst

        let mut cells_compute_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            cells_compute_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: cells_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cells_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: Some(&format!("compute bind group {i}")),
            }));
        }

        // bind group for draw shader

        let colormap_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Colormap Buffer"),
            contents: bytemuck::cast_slice(&state.colormap),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let draw_bind_groups = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: colormap_buffer.as_entire_binding(),
                },
            ],
            label: Some("draw bind group"),
        });

        // calculates number of work groups from CELLS_PER_GROUP constant
        let work_group_count =
            ((state.total_cell_number as f32) / (CELLS_PER_GROUP as f32)).ceil() as u32;

        Self {
            window,

            surface,
            surface_config: config,

            device,
            queue,

            cells_compute_bind_groups,
            draw_bind_groups,

            cells_buffers,
            vertices_buffer,

            compute_pipeline,
            render_pipeline,

            work_group_count,
            frame_num: 0,
            since_last_frame: Instant::now(),
        }
    }

    pub fn render(&mut self, view: &TextureView, state: &AppState) {
        // create render pass descriptor and its color attachments
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                // Not clearing here in order to test wgpu's zero texture initialization on a surface texture.
                // Users should avoid loading uninitialized memory since this can cause additional overhead.
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        // get command encoder
        let mut command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        command_encoder.push_debug_group("compute cell next frame");
        {
            // compute pass
            let mut cpass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.cells_compute_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);
        }
        command_encoder.pop_debug_group();

        command_encoder.push_debug_group("draw cells");
        {
            // render pass
            let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.render_pipeline);
            // uniforms!
            rpass.set_bind_group(0, &self.draw_bind_groups, &[]);
            // give the cell color
            rpass.set_vertex_buffer(0, self.cells_buffers[(self.frame_num + 1) % 2].slice(..));
            // the four instance-local vertices
            rpass.set_vertex_buffer(1, self.vertices_buffer.slice(..));
            rpass.draw(0..6, 0..state.total_cell_number);
        }
        command_encoder.pop_debug_group();

        // update frame count
        self.frame_num += 1;

        // done
        self.queue.submit(Some(command_encoder.finish()));
    }
}
