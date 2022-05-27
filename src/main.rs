mod ray;

use nalgebra::{Vector3, Vector3};

fn write_color(color: Vector3<f32>) {
    println!(
        "{} {} {}",
        (color.x * 255.999) as i32,
        (color.y * 255.999) as i32,
        (color.z * 255.999) as i32,
    );
}

fn ray_color()

fn main() {
    // Image
    const aspect_ratio: f32 = 16.0 / 9.0;
    const image_width: i32 = 400;
    const image_height: i32 = (image_width as f32 / aspect_ratio) as i32;

    // Camera

    let viewport_height = 2.0;
    let viewport_width = aspect_ratio * viewport_height;
    let focal_length = 1.0;

    let origin = Vector3::<f32>::new(0.0, 0.0, 0.0);
    let horizontal = Vector3::new(viewport_width, 0.0, 0.0);
    let vertical = Vector3::new(0.0, viewport_height, 0.0);
    let lower_left_corner = origin
        - (horizontal / 2 as f32)
        - (vertical / 2 as f32)
        - Vector3::new(0.0, 0.0, focal_length);

    println!("P3\n{} {}\n255", image_width, image_height);

    for j in (0..image_height).rev() {
        eprintln!("Scanlines remaining: {}", j);
        for i in 0..image_width {
            let r = i as f32 / (image_width - 1) as f32;
            let g = j as f32 / (image_height - 1) as f32;
            let b = 0.25;

            let pixel_color = Vector3::new(r, g, b);

            write_color(pixel_color);
        }
    }

    eprintln!("Done");
}
