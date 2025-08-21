use std::time::Instant;

use bytemuck::cast;
use glam::{Quat, Vec3};
use image::{Rgb, Rgb32FImage, RgbImage, buffer::ConvertBuffer};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    camera::Camera,
    hittable::{Hittable, HittableType},
    material::Material,
    world::World,
};

mod camera;
mod hittable;
mod material;
mod randomness;
mod ray;
mod world;

const MAX_BOUNCE_COUNT: usize = 10;
const RAYS_PER_PIXEL: usize = 1000;

fn main() {
    let camera = Camera::new(90.0, Vec3::new(0.0, 0.0, -2.0), Quat::IDENTITY);
    let world = World::new(vec![
        // Hittable {
        //     typ: HittableType::Sphere {
        //         position: Vec3::ZERO,
        //         radius: 1.0,
        //     },
        //     material: Material::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0), 3.0, 0.0),
        // },
        // Hittable {
        //     typ: HittableType::Sphere {
        //         position: Vec3::new(-1.0, 0.5, 1.0),
        //         radius: 0.5,
        //     },
        //     material: Material::new(Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO, 0.0, 0.0),
        // },
        // Hittable {
        //     typ: HittableType::Sphere {
        //         position: Vec3::new(1.0, -0.5, -2.0),
        //         radius: 0.25,
        //     },
        //     material: Material::new(Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO, 0.0, 0.0),
        // },
        // Hittable {
        //     typ: HittableType::Sphere {
        //         position: Vec3::new(0.0, -12.0, 0.0),
        //         radius: 10.0,
        //     },
        //     material: Material::new(Vec3::new(1.0, 0.0, 1.0), Vec3::ZERO, 0.0, 1.0),
        // },
        // Hittable {
        //     typ: HittableType::Triangle {
        //         a: Vec3::new(-3.0, 1.0, 0.0),
        //         b: Vec3::new(-2.5, -1.0, 1.0),
        //         c: Vec3::new(-3.5, -1.0, -1.0),
        //     },
        //     material: Material::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, 0.0, 0.5),
        // },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, -1.0, 1.0),
                b: Vec3::new(-1.0, 1.0, 1.0),
                c: Vec3::new(1.0, -1.0, 1.0),
                na: Vec3::NEG_Z,
                nb: Vec3::NEG_Z,
                nc: Vec3::NEG_Z,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, 1.0, 1.0),
                b: Vec3::new(1.0, -1.0, 1.0),
                c: Vec3::new(-1.0, 1.0, 1.0),
                na: Vec3::NEG_Z,
                nb: Vec3::NEG_Z,
                nc: Vec3::NEG_Z,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, -1.0, -1.0),
                b: Vec3::new(-1.0, 1.0, -1.0),
                c: Vec3::new(-1.0, -1.0, 1.0),
                na: Vec3::X,
                nb: Vec3::X,
                nc: Vec3::X,
            },
            material: Material::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, 1.0, 1.0),
                b: Vec3::new(-1.0, -1.0, 1.0),
                c: Vec3::new(-1.0, 1.0, -1.0),
                na: Vec3::X,
                nb: Vec3::X,
                nc: Vec3::X,
            },
            material: Material::new(
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, -1.0, 1.0),
                b: Vec3::new(1.0, 1.0, 1.0),
                c: Vec3::new(1.0, -1.0, -1.0),
                na: Vec3::NEG_X,
                nb: Vec3::NEG_X,
                nc: Vec3::NEG_X,
            },
            material: Material::new(
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, 1.0, -1.0),
                b: Vec3::new(1.0, -1.0, -1.0),
                c: Vec3::new(1.0, 1.0, 1.0),
                na: Vec3::NEG_X,
                nb: Vec3::NEG_X,
                nc: Vec3::NEG_X,
            },
            material: Material::new(
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, -1.0, -1.0),
                b: Vec3::new(1.0, 1.0, -1.0),
                c: Vec3::new(-1.0, -1.0, -1.0),
                na: Vec3::Z,
                nb: Vec3::Z,
                nc: Vec3::Z,
            },
            material: Material::new(
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, 1.0, -1.0),
                b: Vec3::new(-1.0, -1.0, -1.0),
                c: Vec3::new(1.0, 1.0, -1.0),
                na: Vec3::Z,
                nb: Vec3::Z,
                nc: Vec3::Z,
            },
            material: Material::new(
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, -1.0, -1.0),
                b: Vec3::new(-1.0, -1.0, 1.0),
                c: Vec3::new(1.0, -1.0, -1.0),
                na: Vec3::Y,
                nb: Vec3::Y,
                nc: Vec3::Y,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, -1.0, 1.0),
                b: Vec3::new(1.0, -1.0, -1.0),
                c: Vec3::new(-1.0, -1.0, 1.0),
                na: Vec3::Y,
                nb: Vec3::Y,
                nc: Vec3::Y,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-1.0, 1.0, 1.0),
                b: Vec3::new(-1.0, 1.0, -1.0),
                c: Vec3::new(1.0, 1.0, 1.0),
                na: Vec3::NEG_Y,
                nb: Vec3::NEG_Y,
                nc: Vec3::NEG_Y,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(1.0, 1.0, -1.0),
                b: Vec3::new(1.0, 1.0, 1.0),
                c: Vec3::new(-1.0, 1.0, -1.0),
                na: Vec3::NEG_Y,
                nb: Vec3::NEG_Y,
                nc: Vec3::NEG_Y,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(-0.5, 0.99, 0.5),
                b: Vec3::new(-0.5, 0.99, -0.5),
                c: Vec3::new(0.5, 0.99, 0.5),
                na: Vec3::NEG_Y,
                nb: Vec3::NEG_Y,
                nc: Vec3::NEG_Y,
            },
            material: Material::new(
                Vec3::ZERO,
                Vec3::new(1.0, 1.0, 1.0),
                1.0,
                0.0,
                0.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Triangle {
                a: Vec3::new(0.5, 0.99, -0.5),
                b: Vec3::new(0.5, 0.99, 0.5),
                c: Vec3::new(-0.5, 0.99, -0.5),
                na: Vec3::NEG_Y,
                nb: Vec3::NEG_Y,
                nc: Vec3::NEG_Y,
            },
            material: Material::new(
                Vec3::ZERO,
                Vec3::new(1.0, 1.0, 1.0),
                1.0,
                0.0,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(-0.75, -0.75, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.0,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(-0.45, -0.45, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.2,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(-0.15, -0.15, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.4,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(0.15, 0.15, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.6,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(0.45, 0.45, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                0.8,
                1.0,
                Vec3::ONE,
            ),
        },
        Hittable {
            typ: HittableType::Sphere {
                position: Vec3::new(0.75, 0.75, 0.0),
                radius: 0.15,
            },
            material: Material::new(
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::ZERO,
                0.0,
                1.0,
                1.0,
                Vec3::ONE,
            ),
        },
    ]);

    let width = 1080;
    let height = 1080;

    let start = Instant::now();

    let image = Rgb32FImage::from_par_fn(width, height, |x, y| {
        let y = height - y - 1;

        Rgb(cast(
            (0..RAYS_PER_PIXEL)
                .into_par_iter()
                .map(|_| {
                    world.trace(camera.get_ray(
                        x as f32 + fastrand::f32(),
                        y as f32 + fastrand::f32(),
                        width,
                        height,
                    ))
                })
                .sum::<Vec3>()
                / RAYS_PER_PIXEL as f32,
        ))
    });

    println!("it took {:?}", start.elapsed());

    ConvertBuffer::<RgbImage>::convert(&image)
        .save("result.png")
        .unwrap();
}
