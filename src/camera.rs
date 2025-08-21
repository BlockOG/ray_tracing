use derive_more::Constructor;
use glam::{Mat4, Quat, Vec2, Vec3};

use crate::ray::Ray;

#[derive(Debug, Clone, Copy, Constructor)]
pub struct Camera {
    pub field_of_view: f32,
    pub position: Vec3,
    pub rotation: Quat,
}

impl Camera {
    #[inline]
    pub fn world_to_local(&self) -> Mat4 {
        self.local_to_world().inverse()
    }

    #[inline]
    pub fn local_to_world(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }

    #[inline]
    pub fn get_ray(&self, x: f32, y: f32, width: u32, height: u32) -> Ray {
        let uv = Vec2::new(x, y) / Vec2::new(width as f32, height as f32) - 0.5;

        let aspect = width as f32 / height as f32;
        let height = (self.field_of_view / 2.0).to_radians().tan() * 2.0;
        let width = height * aspect;

        Ray::new(
            self.position,
            (self
                .local_to_world()
                .transform_point3(Vec3::new(uv.x * width, uv.y * height, 1.0))
                - self.position)
                .normalize(),
        )
    }
}
