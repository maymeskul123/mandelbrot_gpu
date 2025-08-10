struct Camera {
    pos: vec3<f32>,
    _pad1: f32,
    dir: vec3<f32>,
    _pad2: f32,
    up: vec3<f32>,
    _pad3: f32,
    fov: f32,
    aspect: f32,
    resolution: vec2<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

// Distance Estimation для Мандельбуля
fn mandelbulbDE(p: vec3<f32>) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    let power = 8.0;
    let max_iter = 10u;

    for (var i = 0u; i < max_iter; i = i + 1u) {
        r = length(z);
        if (r > 2.0) {
            break;
        }
        let theta = acos(z.z / r);
        let phi = atan2(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        let zr = pow(r, power);
        let new_theta = theta * power;
        let new_phi = phi * power;

        z = zr * vec3<f32>(
            sin(new_theta) * cos(new_phi),
            sin(new_theta) * sin(new_phi),
            cos(new_theta)
        ) + p;
    }
    return 0.5 * log(r) * r / dr;
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_coord: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(pos[idx], 0.0, 1.0);
    output.frag_coord = pos[idx];
    return output;
}

@fragment
fn fs_main(@location(0) frag_coord: vec2<f32>) -> @location(0) vec4<f32> {
    // Преобразуем из [-1..1] в [0..resolution]
    let uv = (frag_coord + vec2<f32>(1.0, 1.0)) * 0.5 * camera.resolution;

    // Далее нормируем для построения луча
    let ndc = (uv / camera.resolution) * 2.0 - vec2<f32>(1.0, 1.0);

    let forward = normalize(camera.dir);
    let right = normalize(cross(forward, camera.up));
    let up_vec = cross(right, forward);

    let fov_scale = tan(camera.fov * 0.5);
    let ray_dir = normalize(forward + ndc.x * camera.aspect * fov_scale * right + ndc.y * fov_scale * up_vec);

    var t = 0.0;
    let max_dist = 10.0;
    let min_dist = 0.001;
    var hit = false;

    for (var i = 0u; i < 100u; i = i + 1u) {
        let p = camera.pos + ray_dir * t;
        let dist = mandelbulbDE(p);
        if (dist < min_dist) {
            hit = true;
            break;
        }
        if (t > max_dist) {
            break;
        }
        t = t + dist;
    }

    if (hit) {
        let light_dir = normalize(vec3<f32>(-1.0, 1.0, -1.0));
        let p = camera.pos + ray_dir * t;

        let eps = 0.001;
        let nx = mandelbulbDE(p + vec3<f32>(eps, 0.0, 0.0)) - mandelbulbDE(p - vec3<f32>(eps, 0.0, 0.0));
        let ny = mandelbulbDE(p + vec3<f32>(0.0, eps, 0.0)) - mandelbulbDE(p - vec3<f32>(0.0, eps, 0.0));
        let nz = mandelbulbDE(p + vec3<f32>(0.0, 0.0, eps)) - mandelbulbDE(p - vec3<f32>(0.0, 0.0, eps));
        let normal = normalize(vec3<f32>(nx, ny, nz));

        let diffuse = max(dot(normal, light_dir), 0.0);

        // Цвет с вариациями, зависит от нормали и расстояния
        let base_color = vec3<f32>(0.4, 0.6, 1.0);
        let color = base_color * diffuse + vec3<f32>(0.1, 0.05, 0.0) * abs(normal.y);

        return vec4<f32>(color, 1.0);
    } else {
        // Фон (чёрный)
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}
