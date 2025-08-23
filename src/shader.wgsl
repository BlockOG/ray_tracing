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
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var acc_struct: acceleration_structure;

@group(0) @binding(2)
var output: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(3)
var<storage, read> vertices: array<Vertex>;

@group(0) @binding(4)
var<storage, read> materials: array<Material>;

@group(0) @binding(5)
var<storage, read> vertex_offsets: array<u32>;

fn hash(seed: u32) -> u32 {
    var res = seed;
    res = (res ^ 61) ^ (res >> 16);
    res *= 9;
    res = res ^ (res >> 4);
    res *= 0x27d4eb2d;
    res = res ^ (res >> 15);
    return res;
}

fn rand(rng_state: ptr<function, u32>) -> u32 {
    // Xorshift algorithm from George Marsaglia's paper
    *rng_state ^= (*rng_state << 13);
    *rng_state ^= (*rng_state >> 17);
    *rng_state ^= (*rng_state << 5);
    return *rng_state;
}

fn rand_f32(rng_state: ptr<function, u32>) -> f32 {
    return f32(rand(rng_state)) * (1.0 / 4294967296.0);
}

fn erfinv(x: f32) -> f32 {
    let sgn = sign(x);
    let x1 = (1.0 - x) * (1.0 + x);
    let lnx = log(x1);
    let tt1 = 2.0 / (3.14159265359 * 0.147) + 0.5 * lnx;
    let tt2 = 1.0 / (0.147) * lnx;

    return (sgn * sqrt(-tt1 + sqrt(tt1 * tt1 - tt2)));
}

fn rand_vec3(rng_state: ptr<function, u32>) -> vec3<f32> {
    return normalize(vec3<f32>(erfinv(rand_f32(rng_state) * 2.0 - 1.0), erfinv(rand_f32(rng_state) * 2.0 - 1.0), erfinv(rand_f32(rng_state) * 2.0 - 1.0)));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let target_size = textureDimensions(output);
    let pos = vec2<u32>(global_id.x, target_size.y - global_id.y - 1);
    var rng_state = hash(pos.x + pos.y * target_size.x);

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
        var ior = 1.0;
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
                    let normal = normalize((intersection.object_to_world * vec4<f32>(normalize(vertices[primitive_index + 0].normal * u + vertices[primitive_index + 1].normal * v + vertices[primitive_index + 2].normal * w), 0.0)).xyz);
                    let material = materials[intersection.instance_custom_data];

                    let diffuse_direction = normalize(normal + rand_vec3(&rng_state));
                    let specular_direction = reflect(direction, normal);
                    let is_specular = rand_f32(&rng_state) < material.specular_probability;
                    origin = origin + direction * intersection.t;
                    direction = mix(diffuse_direction, specular_direction, material.smoothness * f32(is_specular));

                    let emitted_light = material.emission_color * material.emission_strength;
                    incoming_light += emitted_light * color;
                    if is_specular {
                        color *= material.specular_color;
                    } else {
                        color *= material.color;
                    };
                } else {
                    // if direction.y < 0.0 {
                    //     color *= 0.5;
                    // } else {
                    //     color *= mix(vec3<f32>(0.0, 0.596078431372549, 0.8588235294117647), vec3<f32>(1.0), (1.0 - direction.y) * (1.0 - direction.y));
                    // }

                    // incoming_light += color;
                    break;
                }
            } else {
                break;
            }
        }

        total_color += incoming_light;
        // total_color += color;
    }

    textureStore(output, global_id.xy, vec4<f32>(total_color / f32(uniforms.rays_per_pixel), 1.0));
}