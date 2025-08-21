use derive_more::Constructor;
use glam::Vec3;

#[derive(Debug, Clone, Copy, Constructor)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    #[inline]
    pub fn follow(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}
