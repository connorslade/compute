use anyhow::Result;

use image::{ImageBuffer, Rgb};
use tufa::{
    bindings::buffer::mutability::Mutable,
    export::{
        encase::ShaderType,
        nalgebra::{Vector2, Vector3},
        wgpu::include_wgsl,
    },
    gpu::Gpu,
};

#[derive(ShaderType)]
struct Uniform {
    size: Vector2<u32>,
    zoom: f32,
}

const SIZE: Vector2<u32> = Vector2::new(4096, 4096);

fn main() -> Result<()> {
    let gpu = Gpu::new()?;

    let uniform = gpu.create_uniform(&Uniform {
        size: SIZE,
        zoom: 0.0,
    });
    let buffer = gpu.create_storage_empty::<Vec<u32>, Mutable>((4 * SIZE.x * SIZE.y) as u64);

    let mut pipeline = gpu
        .compute_pipeline(include_wgsl!("shader.wgsl"))
        .bind(&uniform)
        .bind(&buffer)
        .finish();

    for zoom in 0..10_0 {
        uniform.upload(&Uniform {
            size: SIZE,
            zoom: zoom as f32 / 10.0,
        });

        pipeline.dispatch(Vector3::new(SIZE.x / 8, SIZE.y / 8, 1));
        buffer.download_async(move |result| {
            ImageBuffer::from_par_fn(SIZE.x, SIZE.y, |x, y| {
                let color = result[(y * SIZE.x + x) as usize];
                Rgb([color as u8, (color >> 8) as u8, (color >> 16) as u8])
            })
            .save(format!("rec/out-{zoom:0>4}.png"))
            .unwrap();
        });
    }

    Ok(())
}
