use std::ops::Range;

use encase::ShaderType;
use nalgebra::{Vector2, Vector4};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, ColorTargetState, ColorWrites, FragmentState, IndexFormat,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPass, ShaderModule, ShaderModuleDescriptor, ShaderStages, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::{
    buffer::{Bindable, BindableResource, IndexBuffer, VertexBuffer},
    gpu::Gpu,
    misc::ids::PipelineId,
    TEXTURE_FORMAT,
};

pub const VERTEX_BUFFER_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: 32, // NOTE: WGSL alignment rules factor into this
    step_mode: VertexStepMode::Vertex,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: 4 * 4,
            shader_location: 1,
        },
    ],
};

pub const QUAD_INDEX: &[u32] = &[0, 1, 2, 2, 3, 0];
pub const QUAD_VERTEX: &[Vertex] = &[
    Vertex::new(Vector4::new(-1.0, -1.0, 1.0, 1.0), Vector2::new(0.0, 0.0)),
    Vertex::new(Vector4::new(1.0, -1.0, 1.0, 1.0), Vector2::new(1.0, 0.0)),
    Vertex::new(Vector4::new(1.0, 1.0, 1.0, 1.0), Vector2::new(1.0, 1.0)),
    Vertex::new(Vector4::new(-1.0, 1.0, 1.0, 1.0), Vector2::new(0.0, 1.0)),
];

#[derive(ShaderType)]
pub struct Vertex {
    pub position: Vector4<f32>,
    pub uv: Vector2<f32>,
}

impl Vertex {
    pub const fn new(position: Vector4<f32>, uv: Vector2<f32>) -> Self {
        Self { position, uv }
    }
}

pub struct RenderPipeline {
    gpu: Gpu,

    id: PipelineId,
    pipeline: wgpu::RenderPipeline,
    entries: Vec<BindableResource>,
    bind_group: BindGroup,

    // todo: store in Gpu and reuse across pipelines?
    buffers: Option<(VertexBuffer<Vertex>, IndexBuffer)>,
}

pub struct RenderPipelineBuilder {
    gpu: Gpu,

    module: ShaderModule,
    bind_group_layout: Vec<BindGroupLayoutEntry>,
    bind_group: Vec<BindableResource>,
}

impl RenderPipeline {
    fn recreate_bind_group(&mut self) {
        if self.gpu.pipelines.read()[&self.id].1 {
            self.bind_group = create_bind_group(&self.gpu, &self.pipeline, &self.entries);
        }
    }

    pub fn draw_indexed(
        &mut self,
        render_pass: &mut RenderPass,
        index: &IndexBuffer,
        vertex: &VertexBuffer<Vertex>,
        indices: Range<u32>,
    ) {
        self.recreate_bind_group();

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, Some(&self.bind_group), &[]);
        render_pass.set_index_buffer(index.get().slice(..), IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, vertex.get().slice(..));
        render_pass.draw_indexed(indices, 0, 0..1);
    }

    pub fn draw_screen_quad(&mut self, render_pass: &mut RenderPass) {
        self.recreate_bind_group();
        let (vertex, index) = self.buffers.get_or_insert_with(|| {
            (
                self.gpu.create_vertex(QUAD_VERTEX).unwrap(),
                self.gpu.create_index(QUAD_INDEX).unwrap(),
            )
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, Some(&self.bind_group), &[]);
        render_pass.set_index_buffer(index.get().slice(..), IndexFormat::Uint32);
        render_pass.set_vertex_buffer(0, vertex.get().slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

impl RenderPipelineBuilder {
    pub fn bind_buffer(mut self, entry: &impl Bindable, visibility: ShaderStages) -> Self {
        let binding = self.bind_group.len() as u32;

        self.bind_group.push(entry.resource());
        self.bind_group_layout.push(BindGroupLayoutEntry {
            binding,
            visibility,
            ty: entry.binding_type(),
            count: None,
        });

        self
    }

    pub fn finish(self) -> RenderPipeline {
        let device = &self.gpu.device;

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &self.bind_group_layout,
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: &self.module,
                entry_point: Some("vert"),
                buffers: &[VERTEX_BUFFER_LAYOUT],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &self.module,
                entry_point: Some("frag"),
                targets: &[Some(ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bind_group = create_bind_group(&self.gpu, &pipeline, &self.bind_group);
        let id = PipelineId::new();
        self.gpu
            .pipelines
            .write()
            .insert(id, (self.bind_group.clone(), false));

        RenderPipeline {
            gpu: self.gpu,
            id,
            pipeline,
            bind_group,
            entries: self.bind_group,

            buffers: None,
        }
    }
}

fn create_bind_group(
    gpu: &Gpu,
    pipeline: &wgpu::RenderPipeline,
    entries: &[BindableResource],
) -> BindGroup {
    let buffers = gpu.buffers.read();
    gpu.device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries
            .iter()
            .enumerate()
            .map(|(binding, id)| BindGroupEntry {
                binding: binding as u32,
                resource: match id {
                    BindableResource::Buffer(buffer) => buffers[buffer].as_entire_binding(),
                },
            })
            .collect::<Vec<_>>(),
    })
}

impl Gpu {
    pub fn render_pipeline(&self, source: ShaderModuleDescriptor) -> RenderPipelineBuilder {
        let module = self.device.create_shader_module(source);

        RenderPipelineBuilder {
            gpu: self.clone(),
            module,
            bind_group_layout: Vec::new(),
            bind_group: Vec::new(),
        }
    }
}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        self.gpu.pipelines.write().remove(&self.id);
    }
}
