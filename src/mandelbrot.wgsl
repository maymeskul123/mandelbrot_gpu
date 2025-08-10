struct Params {
    center_x: f32,
    center_y: f32,
    scale: f32,
    width: u32,
    height: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

fn mandelbrot(c_re: f32, c_im: f32) -> u32 {
    var z_re = 0.0;
    var z_im = 0.0;
    var i = 0u;
    loop {
        if (i == 255u || z_re * z_re + z_im * z_im > 4.0) {
            break;
        }
        let new_re = z_re * z_re - z_im * z_im + c_re;
        let new_im = 2.0 * z_re * z_im + c_im;
        z_re = new_re;
        z_im = new_im;
        i = i + 1u;
    }
    return i;
}

fn julia(z_re: f32, z_im: f32, c_re: f32, c_im: f32) -> u32 {
    var x = z_re;
    var y = z_im;
    var i = 0u;
    loop {
        if (i == 255u || x * x + y * y > 4.0) {
            break;
        }
        let xtemp = x * x - y * y + c_re;
        y = 2.0 * x * y + c_im;
        x = xtemp;
        i = i + 1u;
    }
    return i;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let x = GlobalInvocationID.x;
    let y = GlobalInvocationID.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    let scale = params.scale;
    let center_x = params.center_x;
    let center_y = params.center_y;

    let c_re = center_x + (f32(x) - f32(params.width) * 0.5) * scale / f32(params.width);
    let c_im = center_y + (f32(y) - f32(params.height) * 0.5) * scale / f32(params.height);

    let iter = julia(c_re, c_im, 0.355, 0.355);

    let t = f32(iter) / 255.0;
    let r = 9.0 * (1.0 - t) * t * t * t;
    let g = 15.0 * (1.0 - t) * (1.0 - t) * t * t;
    let b = 8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t;

    textureStore(
        output_tex,
        vec2<i32>(i32(x), i32(y)),
        vec4<f32>(r, g, b, 1.0)
    );
}