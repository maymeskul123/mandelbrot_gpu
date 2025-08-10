struct Camera {
    pos: vec3<f32>,
    _pad1: f32,
    dir: vec3<f32>,
    _pad2: f32,
    up: vec3<f32>,
    _pad3: f32,
    fov: f32,
    aspect: f32,
    _pad4: vec2<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

// Возвращает расстояние до множества Мандельбуля
fn mandelbulbDE(p: vec3<f32>) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    let power = 8.0;
    let max_iter = 10;

    for (var i = 0; i < max_iter; i = i + 1) {
        r = length(z);
        if (r > 2.0) {
            break;
        }

        // преобразование в сферические координаты
        let theta = acos(z.z / r);
        let phi = atan2(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // масштабируем и поворачиваем точку
        let zr = pow(r, power);
        let new_theta = theta * power;
        let new_phi = phi * power;

        // возвращаем в декартовы координаты
        z = zr * vec3<f32>(
            sin(new_theta) * cos(new_phi),
            sin(new_theta) * sin(new_phi),
            cos(new_theta)
        ) + p;
    }

    return 0.5 * log(r) * r / dr;
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    return vec4<f32>(pos[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (frag_coord.xy / vec2<f32>(1920.0, 1080.0)) * 2.0 - vec2<f32>(1.0, 1.0);
    let forward = normalize(camera.dir);
    let right = normalize(cross(forward, camera.up));
    let up_vec = cross(right, forward);

    let fov_scale = tan(camera.fov * 0.5);
    let ray_dir = normalize(forward + uv.x * camera.aspect * fov_scale * right + uv.y * fov_scale * up_vec);

    var t = 0.0;
    let max_dist = 10.0;
    let min_dist = 0.001;
    var hit = false;

    for (var i = 0; i < 100; i = i + 1) {
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
        // Цвет зависит от расстояния для простого эффекта освещения
        let light_dir = normalize(vec3<f32>(-1.0, 1.0, -1.0));
        let p = camera.pos + ray_dir * t;

        // Аппроксимация нормали через градиент
        let eps = 0.001;
        let nx = mandelbulbDE(p + vec3<f32>(eps, 0.0, 0.0)) - mandelbulbDE(p - vec3<f32>(eps, 0.0, 0.0));
        let ny = mandelbulbDE(p + vec3<f32>(0.0, eps, 0.0)) - mandelbulbDE(p - vec3<f32>(0.0, eps, 0.0));
        let nz = mandelbulbDE(p + vec3<f32>(0.0, 0.0, eps)) - mandelbulbDE(p - vec3<f32>(0.0, 0.0, eps));
        let normal = normalize(vec3<f32>(nx, ny, nz));

        let diffuse = max(dot(normal, light_dir), 0.0);
        return vec4<f32>(diffuse, diffuse * 0.5, diffuse * 0.2, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}
