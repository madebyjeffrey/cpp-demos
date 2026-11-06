//! Skia owns GPU resources; the window manager owns drawable acquisition/presentation.

use super::{Error, Result, error};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::{MTLCommandQueue, MTLDevice, MTLTexture};
use skia_safe::{Canvas, ColorType, Surface, gpu};
use std::cell::RefCell;

pub(super) struct Renderer {
    context: RefCell<gpu::DirectContext>,
}

pub(super) struct Target {
    surface: Surface,
    // Keep the backend descriptor (and its retained Metal texture) alive until
    // after the Skia surface has been released.
    _backend: gpu::BackendRenderTarget,
}

impl Renderer {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        queue: &ProtocolObject<dyn MTLCommandQueue>,
    ) -> Result<Self> {
        // SAFETY: Both handles are live Objective-C Metal objects on the same
        // device. BackendContext and DirectContext each retain their own refs.
        let backend = unsafe {
            gpu::mtl::BackendContext::new(
                device as *const _ as gpu::mtl::Handle,
                queue as *const _ as gpu::mtl::Handle,
            )
        };
        let context = gpu::direct_contexts::make_metal(&backend, None)
            .ok_or_else(|| error("Cannot create Skia Metal context"))?;
        Ok(Self {
            context: RefCell::new(context),
        })
    }

    pub fn target(&self, texture: &Retained<ProtocolObject<dyn MTLTexture>>) -> Result<Target> {
        let width =
            i32::try_from(texture.width()).map_err(|_| error("Skia texture is too wide"))?;
        let height =
            i32::try_from(texture.height()).map_err(|_| error("Skia texture is too tall"))?;
        // SAFETY: The drawable supplies a live BGRA8 Metal texture on our device.
        // TextureInfo retains it, as does the backend descriptor created from it.
        let info = unsafe { gpu::mtl::TextureInfo::new(Retained::as_ptr(texture) as _) };
        let backend = gpu::backend_render_targets::make_mtl((width, height), &info);
        let surface = gpu::surfaces::wrap_backend_render_target(
            &mut self.context.borrow_mut(),
            &backend,
            gpu::SurfaceOrigin::TopLeft,
            ColorType::BGRA8888,
            None,
            None,
        )
        .ok_or_else(|| error("Cannot wrap Metal drawable in a Skia surface"))?;
        Ok(Target {
            surface,
            _backend: backend,
        })
    }

    pub fn submit(&self, target: &mut Target) -> Result<()> {
        // Finish any saved layers before submitting, even after a callback panic.
        target.canvas().restore_to_count(1);
        self.context
            .borrow_mut()
            .flush_and_submit_surface(&mut target.surface, None)
            .map(|_| ())
            .map_err(|err| Error(format!("Skia Metal submission failed: {err:?}")))
    }
}

impl Target {
    pub fn canvas(&mut self) -> &Canvas {
        self.surface.canvas()
    }
}
