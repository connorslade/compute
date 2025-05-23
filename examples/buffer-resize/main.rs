use anyhow::Result;

use tufa::{
    bindings::buffer::mutability::Mutable,
    export::{encase::ShaderType, nalgebra::Vector3, wgpu::include_wgsl},
    gpu::Gpu,
};

#[derive(ShaderType)]
struct Data {
    out: f32,
    #[size(runtime)]
    items: Vec<f32>,
}

fn main() -> Result<()> {
    let gpu = Gpu::new()?;

    let buffer = gpu.create_storage::<_, Mutable>(&Data {
        out: 0.0,
        items: vec![1.0, 2.0],
    });

    let mut pipeline = gpu
        .compute_pipeline(include_wgsl!("compute.wgsl"))
        .bind(&buffer)
        .finish();

    buffer.upload(&Data {
        out: 0.0,
        items: vec![1.0, 2.0, 3.0, 4.0],
    });

    pipeline.dispatch(Vector3::new(1, 1, 1));

    let result = buffer.download();
    println!("Result: {}", result.out);

    Ok(())
}
