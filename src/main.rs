use std::{iter, mem, time::Instant};

use bytemuck::{Pod, Zeroable};
use glam::{Affine3A, Mat4, Quat, Vec2, Vec3, Vec3A};
use image::{RgbImage, RgbaImage, buffer::ConvertBuffer};
use speedy::{Readable, Writable};
use wgpu::{Extent3d, util::DeviceExt};

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 1080;

const MAX_BOUNCE_COUNT: usize = 30;
const RAYS_PER_PIXEL: usize = 1000;

#[derive(Debug, Clone, Copy, Readable, Writable)]
struct Camera {
    position: Vec3,
    rotation: Quat,
    field_of_view: f32,
    near: f32,
    far: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Readable, Writable)]
struct Material {
    color: Vec3,
    emission_strength: f32,
    emission_color: Vec3,
    specular_probability: f32,
    specular_color: Vec3,
    smoothness: f32,
}

#[derive(Debug, Clone, Copy, Readable, Writable)]
struct WrittenVertex {
    pos: Vec3,
    tex_coord: Vec2,
    normal: Vec3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: Vec3,
    _p0: [u32; 1],
    tex_coord: Vec2,
    _p1: [u32; 2],
    normal: Vec3,
    _p2: [u32; 1],
}

#[derive(Debug, Clone, Readable, Writable)]
struct Mesh {
    vertices: Vec<WrittenVertex>,
}

#[derive(Debug, Clone, Copy, Readable, Writable)]
struct Instance {
    transform: Affine3A,
    mesh: u32,
    material: u32,
}

#[derive(Debug, Clone, Readable, Writable)]
struct World {
    camera: Camera,
    materials: Vec<Material>,
    meshes: Vec<Mesh>,
    instances: Vec<Instance>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_inverse: Mat4,
    proj_inverse: Mat4,
    max_bounce_count: u32,
    rays_per_pixel: u32,
    _p0: [u32; 2],
}

impl Uniforms {
    fn new(view_inverse: Mat4, proj_inverse: Mat4, max_bounce_count: u32, rays_per_pixel: u32) -> Self {
        Self {
            view_inverse,
            proj_inverse,
            max_bounce_count,
            rays_per_pixel,
            _p0: [0, 0],
        }
    }
}

#[inline]
fn affine_to_rows(mat: &Affine3A) -> [f32; 12] {
    let row_0 = mat.matrix3.row(0);
    let row_1 = mat.matrix3.row(1);
    let row_2 = mat.matrix3.row(2);
    let translation = mat.translation;
    [row_0.x, row_0.y, row_0.z, translation.x, row_1.x, row_1.y, row_1.z, translation.y, row_2.x, row_2.y, row_2.z, translation.z]
}

fn main() {
    let world = World::read_from_file("scene").unwrap();

    let start = Instant::now();

    let image = {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::EXPERIMENTAL_RAY_QUERY | wgpu::Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE,
            required_limits: wgpu::Limits::default().using_minimum_supported_acceleration_structure_values(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let uniforms = {
            let view = Mat4::look_to_rh(world.camera.position, world.camera.rotation * Vec3::NEG_Z, Vec3::Y);
            let proj = Mat4::perspective_rh(world.camera.field_of_view.to_radians(), WIDTH as f32 / HEIGHT as f32, world.camera.near, world.camera.far);
            Uniforms::new(view.inverse(), proj.inverse(), MAX_BOUNCE_COUNT as u32, RAYS_PER_PIXEL as u32)
        };

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&world.materials),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(
                &world
                    .meshes
                    .iter()
                    .flat_map(|mesh| mesh.vertices.iter())
                    .map(|vertex| Vertex {
                        pos: vertex.pos,
                        _p0: [0],
                        tex_coord: vertex.tex_coord,
                        _p1: [0, 0],
                        normal: vertex.normal,
                        _p2: [0],
                    })
                    .collect::<Vec<Vertex>>(),
            ),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });
        let vertex_offset_buffer = {
            let mesh_offsets = world
                .meshes
                .iter()
                .scan(0, |vertex_offset, mesh| {
                    let start_vertex_offset = *vertex_offset;
                    *vertex_offset += mesh.vertices.len() as u32;
                    Some(start_vertex_offset)
                })
                .collect::<Vec<u32>>();
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&world.instances.iter().map(|instance| mesh_offsets[instance.mesh as usize]).collect::<Vec<u32>>()),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });
        let output_texture_view = output_texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (WIDTH * 4).next_multiple_of(256) as u64 * HEIGHT as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let blases: Vec<_> = world
            .meshes
            .iter()
            .map(|mesh| {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&mesh.vertices.iter().map(|vertex| vertex.pos.into()).collect::<Vec<Vec3A>>()),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::BLAS_INPUT,
                });
                let blas_geometry_size_description = wgpu::BlasTriangleGeometrySizeDescriptor {
                    vertex_format: wgpu::VertexFormat::Float32x3,
                    vertex_count: mesh.vertices.len() as u32 / 3,
                    index_format: None,
                    index_count: None,
                    flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
                };
                let blas = device.create_blas(
                    &wgpu::CreateBlasDescriptor {
                        label: None,
                        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                    },
                    wgpu::BlasGeometrySizeDescriptors::Triangles {
                        descriptors: vec![blas_geometry_size_description.clone()],
                    },
                );

                (blas_geometry_size_description, blas, vertex_buffer)
            })
            .collect();
        let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: None,
            max_instances: world.instances.len() as u32,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        });
        for (i, instance) in world.instances.iter().enumerate() {
            tlas[i] = Some(wgpu::TlasInstance::new(&blases[instance.mesh as usize].1, affine_to_rows(&instance.transform), instance.material, 0xff));
        }

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::AccelerationStructure { vertex_return: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::AccelerationStructure(&tlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&output_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: vertex_offset_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.build_acceleration_structures(
            blases
                .iter()
                .map(|(size, blas, vertex_buffer)| wgpu::BlasBuildEntry {
                    blas,
                    geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                        size,
                        vertex_buffer,
                        first_vertex: 0,
                        vertex_stride: mem::size_of::<Vec3A>() as u64,
                        index_buffer: None,
                        first_index: None,
                        transform_buffer: None,
                        transform_buffer_offset: None,
                    }]),
                })
                .collect::<Vec<_>>()
                .iter(),
            iter::once(&tlas),
        );

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, Some(&bind_group), &[]);
            pass.dispatch_workgroups(WIDTH.div_ceil(8), HEIGHT.div_ceil(8), 1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &download_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some((WIDTH * 4).next_multiple_of(256)),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        let download_buffer = download_buffer.slice(..);
        download_buffer.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::Wait).unwrap();

        let data = download_buffer.get_mapped_range();
        let data: &[u8] = bytemuck::cast_slice(&data);
        RgbaImage::from_raw(
            WIDTH,
            HEIGHT,
            data.chunks((WIDTH as usize * 4).next_multiple_of(256)).flat_map(|b| b.into_iter().map(|b| *b).take(WIDTH as usize * 4)).collect(),
        )
        .unwrap()
    };

    println!("it took {:?}", start.elapsed());

    ConvertBuffer::<RgbImage>::convert(&image).save("result.png").unwrap();
}
