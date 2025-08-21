use derive_more::Constructor;
use glam::Vec3;

use crate::{
    MAX_BOUNCE_COUNT,
    hittable::{HitInfo, Hittable},
    randomness,
    ray::Ray,
};

#[derive(Debug, Clone, Constructor)]
pub struct World {
    pub objects: Vec<Hittable>,
}

impl World {
    pub fn intersect(&self, ray: Ray) -> Option<HitInfo> {
        let mut closest_hit: Option<HitInfo> = None;
        for object in &self.objects {
            if let Some(hit) = object.intersect(ray) {
                if closest_hit
                    .map(|closest_hit| hit.distance < closest_hit.distance)
                    .unwrap_or(true)
                {
                    closest_hit = Some(hit);
                }
            }
        }

        closest_hit
    }

    pub fn trace(&self, mut ray: Ray) -> Vec3 {
        let mut incoming_light = Vec3::new(0.0, 0.0, 0.0);
        let mut color = Vec3::new(1.0, 1.0, 1.0);

        for _ in 0..=MAX_BOUNCE_COUNT {
            if color != Vec3::ZERO {
                if let Some(hit) = self.intersect(ray) {
                    let diffuse_direction = (hit.normal + randomness::vector3()).normalize();
                    let specular_direction =
                        ray.direction - 2.0 * ray.direction.dot(hit.normal) * hit.normal;
                    let is_specular = fastrand::f32() < hit.material.specular_probability;
                    ray = Ray::new(
                        hit.position,
                        diffuse_direction.lerp(
                            specular_direction,
                            hit.material.smoothness * is_specular as i32 as f32,
                        ),
                    );

                    let emitted_light =
                        hit.material.emission_color * hit.material.emission_strength;
                    incoming_light += emitted_light * color;
                    color *= if is_specular {
                        hit.material.specular_color
                    } else {
                        hit.material.color
                    };
                } else {
                    if ray.direction.y < 0.0 {
                        color *= 0.5;
                    } else {
                        color *= Vec3::new(0.0, 0.596078431372549, 0.8588235294117647)
                            .lerp(Vec3::ONE, (1.0 - ray.direction.y) * (1.0 - ray.direction.y));
                    }

                    incoming_light += color;

                    break;
                }
            } else {
                break;
            }
        }

        incoming_light
    }
}
