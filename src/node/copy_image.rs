use rendy::{
    command::{
        CommandBuffer, CommandPool, ExecutableState, Family, Families, FamilyId, Fence, MultiShot,
        PendingState, Queue, SimultaneousUse, Submission, Submit, Supports, Transfer,
    },
    factory::{Factory, ImageState},
    frame::Frames,
    graph::{
        gfx_acquire_barriers, gfx_release_barriers, BufferAccess, BufferId, DynNode, GraphContext,
        ImageAccess, ImageId, NodeBuffer, NodeBuilder, NodeId, NodeImage, NodeBuildError,
    },
    texture::Texture,
};

use rendy::hal;
use crate::Aux;

#[derive(Debug)]
pub struct CopyToTexture<B: hal::Backend> {
    pool: CommandPool<B, hal::queue::QueueType>,
    submit: Submit<B, SimultaneousUse>,
    buffer:
        CommandBuffer<B, hal::queue::QueueType, PendingState<ExecutableState<MultiShot<SimultaneousUse>>>>,
}

impl<B: hal::Backend> CopyToTexture<B> {
    pub fn builder(input: ImageId) -> CopyToTextureBuilder {
        CopyToTextureBuilder {
            input,
            dependencies: vec![],
        }
    }
}

#[derive(Debug)]
pub struct CopyToTextureBuilder {
    input: ImageId,
    dependencies: Vec<NodeId>,
}

impl CopyToTextureBuilder {
    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn add_dependency(&mut self, dependency: NodeId) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add dependency.
    /// Node will be placed after its dependencies.
    pub fn with_dependency(mut self, dependency: NodeId) -> Self {
        self.add_dependency(dependency);
        self
    }
}

impl<B> NodeBuilder<B, Aux<B>> for CopyToTextureBuilder
where
    B: hal::Backend,
{
    fn family(&self, _factory: &mut Factory<B>, families: &Families<B>) -> Option<FamilyId> {
        families.find(|family| Supports::<Transfer>::supports(&family.capability()).is_some())
    }

    fn buffers(&self) -> Vec<(BufferId, BufferAccess)> {
        Vec::new()
    }

    fn images(&self) -> Vec<(ImageId, ImageAccess)> {
        vec![(
            self.input,
            ImageAccess {
                access: hal::image::Access::TRANSFER_WRITE,
                layout: hal::image::Layout::TransferDstOptimal,
                usage: hal::image::Usage::TRANSFER_DST,
                stages: hal::pso::PipelineStage::TRANSFER,
            },
        )]
    }

    fn dependencies(&self) -> Vec<NodeId> {
        self.dependencies.clone()
    }

    fn build<'a>(
        self: Box<Self>,
        ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        family: &mut Family<B>,
        queue: usize,
        aux: &Aux<B>,
        buffers: Vec<NodeBuffer>,
        images: Vec<NodeImage>,
    ) -> Result<Box<dyn DynNode<B, Aux<B>>>, NodeBuildError> {
        assert_eq!(buffers.len(), 0);
        assert_eq!(images.len(), 1);

        let mut pool = factory
            .create_command_pool(family)
            .map_err(|e| { 
                log::error!("{}", e); 
                NodeBuildError::OutOfMemory(hal::device::OutOfMemory::Device) // TODO: Wrong error type
            })?;

        let buf_initial = pool.allocate_buffers(1).pop().unwrap();
        let mut buf_recording = buf_initial.begin(MultiShot(SimultaneousUse), ());
        let mut encoder = buf_recording.encoder();
        let buffer = aux.source_buffer.as_ref().unwrap();

        // TODO: Memory barrier
        //{
        //    let buffers = vec![buffer];
        //    let (stages, barriers) = gfx_acquire_barriers(ctx, None, buffers.iter());
        //    log::trace!("Acquire {:?} : {:#?}", stages, barriers);
        //    if !barriers.is_empty() {
        //        encoder.pipeline_barrier(stages, hal::memory::Dependencies::empty(), barriers);
        //    }
        //}

        let image = ctx.get_image(images[0].id).unwrap();
        let image_extent = image.kind().extent();
        unsafe{
            encoder.copy_buffer_to_image(
                buffer.raw(),
                image.raw(),
                images[0].layout,
                Some(hal::command::BufferImageCopy {
                    buffer_offset: 0,
                        buffer_width: image_extent.width,
                        buffer_height: image_extent.height,
                        image_layers: hal::image::SubresourceLayers {
                            aspects: hal::format::Aspects::COLOR,
                            level: 0,
                            layers: 0..1,
                        },
                        image_offset: hal::image::Offset { x: 0, y: 0, z: 0},
                        image_extent: hal::image::Extent { 
                            width: image_extent.width,
                            height: image_extent.height,
                            depth: image_extent.depth,
                        },
                }),
            );
        }

       // {
       //     let (mut stages, mut barriers) = gfx_release_barriers(ctx, None, images.iter());
       //     let end_state = ImageState {
       //         queue: family.queue(queue).id(),
       //         stage: hal::pso::PipelineStage::FRAGMENT_SHADER,
       //         access: hal::image::Access::SHADER_READ,
       //         layout: hal::image::Layout::ShaderReadOnlyOptimal,
       //     };
       //     stages.start |= hal::pso::PipelineStage::TRANSFER;
       //     stages.end |= end_state.stage;
       //     barriers.push(hal::memory::Barrier::Image {
       //         states: (
       //             hal::image::Access::TRANSFER_WRITE,
       //             hal::image::Layout::TransferDstOptimal,
       //         )..(end_state.access, end_state.layout),
       //         families: None,
       //         target: image.raw(),
       //         range: hal::image::SubresourceRange {
       //             aspects: hal::format::Aspects::COLOR,
       //             levels: 0..1,
       //             layers: 0..1,
       //         },
       //     });
//
       //     log::trace!("Release {:?} : {:#?}", stages, barriers);
       //     unsafe{
       //         encoder.pipeline_barrier(stages, hal::memory::Dependencies::empty(), barriers);
       //     }
       // }

        let (submit, buffer) = buf_recording.finish().submit();

        Ok(Box::new(CopyToTexture {
            pool,
            submit,
            buffer,
        }))
    }
}

impl<B> DynNode<B, Aux<B>> for CopyToTexture<B>
where
    B: hal::Backend,
{
    unsafe fn run<'a>(
        &mut self,
        _ctx: &GraphContext<B>,
        _factory: &Factory<B>,
        queue: &mut Queue<B>,
        _aux: &Aux<B>,
        _frames: &Frames<B>,
        waits: &[(&'a B::Semaphore, hal::pso::PipelineStage)],
        signals: &[&'a B::Semaphore],
        fence: Option<&mut Fence<B>>,
    ) {
        queue.submit(
            Some(
                Submission::new()
                    .submits(Some(&self.submit))
                    .wait(waits.iter().cloned())
                    .signal(signals.iter()),
            ),
            fence,
        );
    }

    unsafe fn dispose(mut self: Box<Self>, factory: &mut Factory<B>, _aux: &Aux<B>) {
        drop(self.submit);
        self.pool.free_buffers(Some(self.buffer.mark_complete()));
        factory.destroy_command_pool(self.pool);
    }
}