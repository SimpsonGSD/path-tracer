use rendy::{
    command::{QueueId, RenderPassEncoder},
    factory::{Factory, ImageState},
    graph::{render::*, GraphContext, ImageAccess, NodeBuffer, NodeImage},
    hal::{pso::DescriptorPool, device::Device as _},
    resource::{
        Buffer, BufferInfo, DescriptorSetLayout, Escape, Filter, Handle, ImageView, ImageViewInfo,
        Sampler, ViewKind, WrapMode,DescriptorSet,
    },
    shader::{PathBufShaderInfo, ShaderKind, SourceLanguage},
    texture::{image::ImageTextureConfig, Texture},
};

use rendy::hal;

use std::mem::size_of;
use std::{fs::File, io::BufReader};

use crate::Aux;

lazy_static::lazy_static! {
    static ref VERTEX: PathBufShaderInfo = PathBufShaderInfo::new(
        std::path::PathBuf::from(crate::application_root_dir()).join("assets/shaders/fullscreen_triangle.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref FRAGMENT: PathBufShaderInfo = PathBufShaderInfo::new(
        std::path::PathBuf::from(crate::application_root_dir()).join("assets/shaders/tonemap.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );

    static ref SHADERS: rendy::shader::ShaderSetBuilder = rendy::shader::ShaderSetBuilder::default()
        .with_vertex(&*VERTEX).unwrap()
        .with_fragment(&*FRAGMENT).unwrap();
}


#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TonemapperArgs {
    pub clear_colour_and_exposure: [f32; 4],
}

impl std::fmt::Display for TonemapperArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Clear Colour{}, Exposure: {}", self.clear_colour_and_exposure[0], self.clear_colour_and_exposure[3])
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UniformArgs {
    tonemapper: TonemapperArgs,
}

#[derive(Debug, PartialEq, Eq)]
struct Settings {
    hw_alignment: u64,
}

impl From<&Aux> for Settings {
    fn from(aux: &Aux) -> Self {
        Self::from_aux(aux)
    }
}

impl From<&mut Aux> for Settings {
    fn from(aux: &mut Aux) -> Self {
        Self::from_aux(aux)
    }
}

impl Settings {
    const UNIFORM_SIZE: u64 = size_of::<UniformArgs>() as u64;

    fn from_aux(aux: &Aux) -> Self {
        Settings { hw_alignment: aux.hw_alignment }
    }

    #[inline]
    fn buffer_frame_size(&self) -> u64 {
        ((Self::UNIFORM_SIZE - 1) / self.hw_alignment + 1) * self.hw_alignment
    }

    #[inline]
    fn uniform_offset(&self, index: u64) -> u64 {
        self.buffer_frame_size() * index as u64
    }
}

#[derive(Debug, Default)]
pub struct PipelineDesc;

#[derive(Debug)]
pub struct Pipeline<B: hal::Backend> {
    buffer: Escape<Buffer<B>>,
    //sets: Vec<B::DescriptorSet>,
    //descriptor_pool: B::DescriptorPool,
    descriptor_set: Escape<DescriptorSet<B>>,
    texture: Texture<B>,
    settings: Settings,
}


impl<B> SimpleGraphicsPipelineDesc<B, Aux> for PipelineDesc
where
    B: hal::Backend,
{
    type Pipeline = Pipeline<B>;

    fn images(&self) -> Vec<ImageAccess> {
        vec![ImageAccess {
            access: hal::image::Access::SHADER_READ,
            usage: hal::image::Usage::SAMPLED,
            layout: hal::image::Layout::ShaderReadOnlyOptimal,
            stages: hal::pso::PipelineStage::FRAGMENT_SHADER,
        }]
    }

    fn depth_stencil(&self) -> Option<hal::pso::DepthStencilDesc> {
        None
    }

    fn load_shader_set(
        &self,
        factory: &mut Factory<B>,
        _aux: &Aux,
    ) -> rendy::shader::ShaderSet<B> {
        SHADERS.build(factory, Default::default()).unwrap()
    }

    fn layout(&self) -> Layout {
        Layout {
            sets: vec![SetLayout {
                bindings: vec![
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: hal::pso::DescriptorType::SampledImage,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 1,
                        ty: hal::pso::DescriptorType::Sampler,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                    hal::pso::DescriptorSetLayoutBinding {
                        binding: 2,
                        ty: hal::pso::DescriptorType::UniformBuffer,
                        count: 1,
                        stage_flags: hal::pso::ShaderStageFlags::FRAGMENT,
                        immutable_samplers: false,
                    },
                ],
            }],
            push_constants: Vec::new(),
        }
    }

    fn build<'a>(
        self,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        aux: &Aux,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
        set_layouts: &[Handle<DescriptorSetLayout<B>>],
    ) -> Result<Pipeline<B>, hal::pso::CreationError> {
        assert!(buffers.is_empty());
        //assert!(images.len() == 1);
        assert!(set_layouts.len() == 1);

        let frames = aux.frames;
        let settings: Settings = (&*aux).into();

        // This is how we can load an image and create a new texture.
        let image_reader = BufReader::new(
            File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/textures/logo.png"
            ))
            .map_err(|e| {
                log::error!("Unable to open {}: {:?}", "/assets/textures/logo.png", e);
                hal::pso::CreationError::Other
            })?,
        );

        let texture_builder = rendy::texture::image::load_from_image(
            image_reader,
            ImageTextureConfig {
                generate_mips: false,
                ..Default::default()
            },
        )
        .map_err(|e| { 
            log::error!("Unable to load image: {:?}", e);
            hal::pso::CreationError::Other
        })?;

        let descriptor_set = factory
            .create_descriptor_set(set_layouts[0].clone())
            .unwrap();

        let texture = texture_builder
            .build(
                ImageState {
                    queue,
                    stage: hal::pso::PipelineStage::FRAGMENT_SHADER,
                    access: hal::image::Access::SHADER_READ,
                    layout: hal::image::Layout::ShaderReadOnlyOptimal,
                },
                factory,
            )
            .unwrap();

        let buffer = factory.create_buffer(
            BufferInfo {
                size: settings.buffer_frame_size() * aux.frames as u64,
                usage: hal::buffer::Usage::UNIFORM,
            },
            rendy::memory::MemoryUsageValue::Dynamic,
        )
        .map_err(|e| {
            log::error!("Unable to create uniform buffer: {:?}", e);
            hal::pso::CreationError::Other
        })?;

        unsafe {
            factory.device().write_descriptor_sets(vec![
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 0,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Image(
                        texture.view().raw(),
                        hal::image::Layout::ShaderReadOnlyOptimal,
                    )],
                },
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 1,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Sampler(texture.sampler().raw())],
                },
                hal::pso::DescriptorSetWrite {
                    set: descriptor_set.raw(),
                    binding: 2,
                    array_offset: 0,
                    descriptors: vec![hal::pso::Descriptor::Buffer(
                        buffer.raw(),
                        Some(settings.uniform_offset(0))
                            ..Some(
                                settings.uniform_offset(0) + Settings::UNIFORM_SIZE,
                            ),
                    )],
                },
            ]);
        }

       // let mut descriptor_pool = unsafe {
       //     factory.create_descriptor_pool(
       //         frames,
       //         vec![
       //            hal::pso::DescriptorRangeDesc {
       //                ty: hal::pso::DescriptorType::Sampler,
       //                count: frames,
       //            },
       //            hal::pso::DescriptorRangeDesc {
       //                ty: hal::pso::DescriptorType::SampledImage,
       //                count: frames,
       //            },
       //             hal::pso::DescriptorRangeDesc {
       //                 ty: hal::pso::DescriptorType::UniformBuffer,
       //                 count: frames,
       //             },
       //         ],
       //         hal::pso::DescriptorPoolCreateFlags::empty(),
       //     )?
       // };
//

       //let image_view = factory
       //    .create_image_view(
       //        image_handle.clone(),
       //        ImageViewInfo {
       //            view_kind: ViewKind::D2,
       //            format: hal::format::Format::Rgba32Sfloat,
       //            swizzle: hal::format::Swizzle::NO,
       //            range: images[0].range.clone(),
       //        },
       //    )
       //    .expect("Could not create tonemapper input image view");

        

        

       //unsafe {
       //    factory.device().write_descriptor_sets(vec![
       //        hal::pso::DescriptorSetWrite {
       //            set: descriptor_set.raw(),
       //            binding: 0,
       //            array_offset: 0,
       //            descriptors: vec![hal::pso::Descriptor::Image(
       //                texture.view().raw(),
       //                hal::image::Layout::ShaderReadOnlyOptimal,
       //            )],
       //        },
       //        hal::pso::DescriptorSetWrite {
       //            set: descriptor_set.raw(),
       //            binding: 1,
       //            array_offset: 0,
       //            descriptors: vec![hal::pso::Descriptor::Sampler(texture.sampler().raw())],
       //        },
       //    ]);
       //}

       //let mut sets = Vec::with_capacity(frames);
       //for index in 0..frames {
       //    unsafe {
       //        let set = descriptor_pool
       //            .allocate_set(&set_layouts[0].raw())
       //            .map_err(|e| {
       //                log::error!("Unable to create descriptor pool: {:?}", e);
       //                hal::pso::CreationError::Other
       //            })?;
       //        factory.device().write_descriptor_sets(vec![
       //           //hal::pso::DescriptorSetWrite {
       //           //    set: &set,
       //           //    binding: 0,
       //           //    array_offset: 0,
       //           //    descriptors: Some(hal::pso::Descriptor::Sampler(image_sampler.raw())),
       //           //},
       //           //hal::pso::DescriptorSetWrite {
       //           //    set: &set,
       //           //    binding: 1,
       //           //    array_offset: 0,
       //           //    descriptors: Some(hal::pso::Descriptor::Image(
       //           //        image_view.raw(),
       //           //        hal::image::Layout::ShaderReadOnlyOptimal,
       //           //    )),
       //           //},
       //            hal::pso::DescriptorSetWrite {
       //                set: &set,
       //                binding: 2,
       //                array_offset: 0,
       //                descriptors: Some(hal::pso::Descriptor::Buffer(
       //                    buffer.raw(),
       //                    Some(settings.uniform_offset(index as u64))
       //                        ..Some(
       //                            settings.uniform_offset(index as u64) + Settings::UNIFORM_SIZE,
       //                        ),
       //                )),
       //            },
       //        ]);
       //        sets.push(set);
       //    }
       //}

        Ok(Pipeline {
            buffer,
            //sets,
            texture,
            //image_view,
            //image_sampler,
            //descriptor_pool,
            descriptor_set,
            settings,
        })
    }
}

impl<B> SimpleGraphicsPipeline<B, Aux> for Pipeline<B>
where
    B: hal::Backend,
{
    type Desc = PipelineDesc;

    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        _set_layouts: &[Handle<DescriptorSetLayout<B>>],
        index: usize,
        aux: &Aux,
    ) -> PrepareResult {
        unsafe {
            factory
                .upload_visible_buffer(
                    &mut self.buffer,
                    self.settings.uniform_offset(index as u64),
                    &[UniformArgs {
                        tonemapper: aux.tonemapper_args,
                    }],
                )
                .unwrap()
        };
        PrepareResult::DrawReuse
    }

    fn draw(
        &mut self,
        layout: &B::PipelineLayout,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _aux: &Aux,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                layout,
                0,
                std::iter::once(self.descriptor_set.raw()),
                //Some(&self.sets[index]),
                std::iter::empty(),
            );
            // This is a trick from Sascha Willems which uses just the gl_VertexIndex
            // to calculate the position and uv coordinates for one full-scren "quad"
            // which is actually just a triangle with two of the vertices positioned
            // correctly off screen. This way we don't need a vertex buffer.
            encoder.draw(0..3, 0..1);
        }
    }

    fn dispose(mut self, factory: &mut Factory<B>, _aux: &Aux) {
        unsafe {
            //self.descriptor_pool.reset();
            //factory.device().destroy_descriptor_pool(self.descriptor_pool);
        }
    }
}