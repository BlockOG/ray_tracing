struct Uniforms {
    view_inv: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
    max_bounce_count: u32,
    rays_per_pixel: u32,
};

struct Vertex {
    pos: vec3<f32>,
    tex_coord: vec2<f32>,
    normal: vec3<f32>,
};

struct Material {
    color: vec3<f32>,
    emission_strength: f32,
    emission_color: vec3<f32>,
    specular_probability: f32,
    specular_color: vec3<f32>,
    smoothness: f32,
    typ: u32,
    ior: f32,
    absorption: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var acc_struct: acceleration_structure;

@group(0) @binding(2)
var output: texture_storage_2d<rgba32float, write>;

@group(0) @binding(3)
var<storage, read> vertices: array<Vertex>;

@group(0) @binding(4)
var<storage, read> materials: array<Material>;

@group(0) @binding(5)
var<storage, read> vertex_offsets: array<u32>;

@group(0) @binding(6)
var prev_output: texture_storage_2d<rgba32float, read>;

@group(0) @binding(7)
var<uniform> frame: u32;

const ETA = 1.57079635;
const PI = 3.1415927;
const TAU = 6.2831855;

fn hash(seed: u32) -> u32 {
    var res = seed;
    res = (res ^ 61) ^ (res >> 16);
    res *= 9;
    res ^= (res >> 4);
    res *= 0x27d4eb2d;
    res ^= (res >> 15);
    return res;
}

fn rand(rng_state: ptr<function, u32>) -> u32 {
    *rng_state ^= (*rng_state << 13);
    *rng_state ^= (*rng_state >> 17);
    *rng_state ^= (*rng_state << 5);
    return *rng_state;
}

fn rand_f32(rng_state: ptr<function, u32>) -> f32 {
    return f32(rand(rng_state)) * (1.0 / 4294967296.0);
}

fn normal_dist_2d(u: vec2<f32>) -> vec2<f32> {
    let a = sqrt(-log(u.x));
    let b = TAU * u.y;

    return vec2<f32>(cos(b), sin(b)) * a;
}

fn rand_vec3(rng_state: ptr<function, u32>) -> vec3<f32> {
    let u1 = rand_f32(rng_state);
    let u2 = rand_f32(rng_state);

    let phi = acos(2.0 * u1 - 1.0) - ETA;
    let lambda = TAU * u2;
    return vec3<f32>(cos(phi) * cos(lambda), cos(phi) * sin(lambda), sin(phi));
}

fn reflectance(direction: vec3<f32>, normal: vec3<f32>, ior_a: f32, ior_b: f32) -> f32 {
    let ior_ratio = ior_a / ior_b;
    let cos_in = -dot(direction, normal);
    let sin_sqr_refract = ior_ratio * ior_ratio * (1.0 - cos_in * cos_in);
    if sin_sqr_refract >= 1.0 {
        return 1.0;
    }

    let cos_refract = sqrt(1.0 - sin_sqr_refract);
    let sqrt_ray_perp = (ior_a * cos_in - ior_b * cos_refract) / (ior_a * cos_in + ior_b * cos_refract);
    let sqrt_ray_par = (ior_b * cos_in - ior_a * cos_refract) / (ior_b * cos_in + ior_a * cos_refract);

    return (sqrt_ray_perp * sqrt_ray_perp + sqrt_ray_par * sqrt_ray_par) / 2.0;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let target_size = textureDimensions(output);
    let pos = vec2<u32>(global_id.x, target_size.y - global_id.y - 1);
    var rng_state = hash(pos.x + pos.y * target_size.x + frame * target_size.x * target_size.y);

    var total_color = vec3<f32>(0.0, 0.0, 0.0);
    for (var ray = 0u; ray < uniforms.rays_per_pixel; ray++) {
        let curr_pos = vec2<f32>(pos) + vec2<f32>(rand_f32(&rng_state), rand_f32(&rng_state));
        let in_uv = curr_pos / vec2<f32>(target_size.xy);
        let d = in_uv * 2.0 - 1.0;

        var origin = (uniforms.view_inv * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;
        let temp = uniforms.proj_inv * vec4<f32>(d.x, d.y, 1.0, 1.0);
        var direction = (uniforms.view_inv * vec4<f32>(normalize(temp.xyz), 0.0)).xyz;

        var incoming_light = vec3<f32>(0.0);
        var color = vec3<f32>(1.0);
        var flags = RAY_FLAG_CULL_FRONT_FACING;

        for (var bounce = 0u; bounce <= uniforms.max_bounce_count; bounce++) {
            if any(color != vec3<f32>(0.0)) {
                var rq: ray_query;
                rayQueryInitialize(&rq, acc_struct, RayDesc(flags, 0xFFu, 0.001, 10000.0, origin, direction));
                rayQueryProceed(&rq);

                let intersection = rayQueryGetCommittedIntersection(&rq);
                if intersection.kind != RAY_QUERY_INTERSECTION_NONE {
                    let w = intersection.barycentrics.y;
                    let v = intersection.barycentrics.x;
                    let u = 1.0 - w - v;

                    let primitive_index = vertex_offsets[intersection.instance_index] + intersection.primitive_index * 3;
                    let uv = vertices[primitive_index + 0].tex_coord * u + vertices[primitive_index + 1].tex_coord * v + vertices[primitive_index + 2].tex_coord * w;
                    var normal = normalize((intersection.object_to_world * vec4<f32>(normalize(vertices[primitive_index + 0].normal * u + vertices[primitive_index + 1].normal * v + vertices[primitive_index + 2].normal * w), 0.0)).xyz);
                    let material = materials[intersection.instance_custom_data];

                    if intersection.front_face {
                        normal = -normal;
                    }

                    origin = origin + direction * intersection.t;

                    let diffuse_direction = normalize(normal + rand_vec3(&rng_state));
                    let reflected_direction = reflect(direction, normal);

                    if material.typ == 0 {
                        let is_specular = f32(rand_f32(&rng_state) < material.specular_probability);
                        direction = mix(diffuse_direction, reflected_direction, material.smoothness * is_specular);

                        let emitted_light = material.emission_color * material.emission_strength;
                        incoming_light += emitted_light * color;
                        color *= mix(material.color, material.specular_color, is_specular);
                    } else if material.typ == 1 {
                        var ior_a = select(1.0, material.ior, intersection.front_face);
                        var ior_b = select(material.ior, 1.0, intersection.front_face);
                        let refracted_direction = refract(direction, normal, ior_a / ior_b);
                        let is_reflected = rand_f32(&rng_state) < reflectance(direction, normal, ior_a, ior_b);
                        direction = select(mix(-diffuse_direction, refracted_direction, material.smoothness), mix(diffuse_direction, reflected_direction, material.smoothness), is_reflected);

                        flags = select(RAY_FLAG_CULL_FRONT_FACING, RAY_FLAG_CULL_BACK_FACING, !is_reflected && !intersection.front_face);

                        if intersection.front_face {
                            color *= exp(-intersection.t * (1.0 - material.color) * material.absorption);
                        }
                    }
                } else {
                    if direction.y < 0.0 {
                        color *= 0.5;
                    } else {
                        color *= mix(vec3<f32>(0.0, 0.596078431372549, 0.8588235294117647), vec3<f32>(1.0), (1.0 - direction.y) * (1.0 - direction.y));
                    }

                    incoming_light += color;
                    break;
                }
            } else {
                break;
            }
        }

        total_color += incoming_light; // (1.0 + incoming_light);
    }

    total_color /= f32(uniforms.rays_per_pixel);
    total_color += textureLoad(prev_output, global_id.xy).xyz * f32(frame);
    textureStore(output, global_id.xy, vec4<f32>(total_color / f32(frame + 1), 1.0));
}