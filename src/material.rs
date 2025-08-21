use derive_more::Constructor;
use glam::Vec3;

#[derive(Debug, Clone, Copy, Constructor)]
pub struct Material {
    pub color: Vec3,
    pub emission_color: Vec3,
    pub emission_strength: f32,
    pub smoothness: f32,
    pub specular_probability: f32,
    pub specular_color: Vec3,
}
