use glam::Vec3;

pub fn vector3() -> Vec3 {
    let mut res = Vec3::ONE;
    while res.length_squared() > 1.0 {
        res = Vec3::new(
            fastrand_contrib::f32_range(-1.0..1.0),
            fastrand_contrib::f32_range(-1.0..1.0),
            fastrand_contrib::f32_range(-1.0..1.0),
        );
    }
    res.normalize()
}
