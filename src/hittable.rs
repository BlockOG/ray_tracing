use glam::Vec3;

use crate::{material::Material, ray::Ray};

#[derive(Debug, Clone, Copy)]
pub struct HitInfo {
    pub distance: f32,
    pub position: Vec3,
    pub normal: Vec3,
    pub material: Material,
}

#[derive(Debug, Clone, Copy)]
pub enum HittableType {
    Sphere {
        position: Vec3,
        radius: f32,
    },
    Triangle {
        a: Vec3,
        b: Vec3,
        c: Vec3,
        na: Vec3,
        nb: Vec3,
        nc: Vec3,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Hittable {
    pub typ: HittableType,
    pub material: Material,
}

impl Hittable {
    pub fn intersect(&self, ray: Ray) -> Option<HitInfo> {
        match self.typ {
            HittableType::Sphere { position, radius } => {
                let offset_ray = Ray::new(ray.origin - position, ray.direction);

                let a = /*ray.direction.dot(ray.direction)*/ 1.0;
                let b = 2.0 * offset_ray.origin.dot(offset_ray.direction);
                let c = offset_ray.origin.dot(offset_ray.origin) - radius * radius;
                let d = b * b - 4.0 * a * c;

                (d >= 0.0)
                    .then(|| {
                        let distance = (-b - d.sqrt()) / (2.0 * a);

                        (distance >= 0.0).then(|| {
                            let hit_position = ray.follow(distance);
                            HitInfo {
                                distance,
                                position: hit_position,
                                normal: (hit_position - position).normalize(),
                                material: self.material,
                            }
                        })
                    })
                    .flatten()
            }
            HittableType::Triangle {
                a,
                b,
                c,
                na,
                nb,
                nc,
            } => {
                let edge_ab = b - a;
                let edge_ac = c - a;
                let normal_vector = edge_ab.cross(edge_ac);
                let ao = ray.origin - a;
                let dao = ao.cross(ray.direction);

                let determinant = -ray.direction.dot(normal_vector);
                let inverse_determinant = 1.0 / determinant;

                let distance = ao.dot(normal_vector) * inverse_determinant;
                let u = edge_ac.dot(dao) * inverse_determinant;
                let v = -edge_ab.dot(dao) * inverse_determinant;
                let w = 1.0 - u - v;

                (determinant >= f32::EPSILON && distance >= 0.0 && u >= 0.0 && v >= 0.0 && w >= 0.0)
                    .then(|| HitInfo {
                        distance,
                        position: ray.follow(distance),
                        normal: (na * w + nb * u + nc * v).normalize(),
                        material: self.material,
                    })
            }
        }
    }
}
