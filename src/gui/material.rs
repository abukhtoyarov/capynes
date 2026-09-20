use super::utils::Vertex;

const SHADER : &str = r#"
@group(0) @binding(0) var my_texture: texture_2d<f32>;
@group(0) @binding(1) var my_sampler: sampler;

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) a_pos: vec2<f32>) -> VertexOut {
    var out: VertexOut;
    out.pos = vec4<f32>(a_pos, 0.0, 1.0);
    out.uv = (a_pos + vec2<f32>(1.0,1.0)) * 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let flipped_uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    let tex_color = textureSample(my_texture, my_sampler, flipped_uv);
    return mix(vec4<f32>(0.0, 0.0, 0.0, 1.0), tex_color, tex_color.a);
}
"#;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub size: wgpu::Extent3d,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
}

impl Texture {
    fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, size: &winit::dpi::PhysicalSize<u32>) -> Texture
    {
        log::info!("+ Creating texture...");
        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let format = config.format;

        Self{texture, size, view, format}
    }
}

pub struct Pipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_layout: wgpu::BindGroupLayout,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, shader: &wgpu::ShaderModule) -> Pipeline {
        log::info!("+ Creating pipeline...");
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { multisampled: false,
                                                     view_dimension: wgpu::TextureViewDimension::D2,
                                                     sample_type: wgpu::TextureSampleType::Float { filterable: true } },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        Self{pipeline, bind_layout}
    }
}

pub struct Canvas {
    pub texture: Texture,
    pub pipeline: Pipeline,
    pub bind_group: wgpu::BindGroup
}

impl Canvas {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, size: &winit::dpi::PhysicalSize<u32>) -> Canvas {
        log::info!("+ Creating canvas...");
        let texture = Texture::new(device, config, size);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        
        let pipeline = Pipeline::new(device, config, &shader);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pipeline.bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&texture.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
            label: Some("texture_bind_group"),
        });

        Self{ texture, pipeline, bind_group }
    }

    pub fn write(&mut self, queue: &wgpu::Queue, data: &[u8]) {
        let bytes_per_pixel = self.texture.format.block_size(None).expect("Texture block size does not exist");
        let bpr = bytes_per_pixel * self.texture.size.width;
        let _required_size = (bpr * self.texture.size.height) as usize;

        assert!(
            data.len() >= _required_size,
            "Input buffer too small: got {} bytes, but texture requires {} bytes",
            data.len(),
            _required_size
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(self.texture.size.height),
            },
            self.texture.size,
        );
    }

}
