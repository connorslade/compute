use wgpu::TextureFormat;

pub mod buffer;
pub mod gpu;
#[cfg(feature = "interactive")]
pub mod interactive;
pub mod misc;
pub mod pipeline;

pub mod export {
    #[cfg(feature = "interactive")]
    pub use {egui, winit};
    pub use {encase, nalgebra, wgpu};
}

const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

// todo storage vs uniform buffers
