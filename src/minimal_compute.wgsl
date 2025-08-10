struct Params {
    center_x: f32,
    center_y: f32,
    scale: f32,
    width: u32,
    height: u32,
    max_iter: u32,
    bytes_per_row_pixels: u32,
    _pad1: u32,
};

@group(0) @binding(0)
var<uniform> params: Params;

@group(0) @binding(1)
var<storage, read_write> output: array<u32>;

fn mandelbrot(c_re: f32, c_im: f32, max_iter: u32) -> u32 {
    var z_re = 0.0;
    var z_im = 0.0;
    var i: u32 = 0u;
    loop {
        if (i >= max_iter || z_re * z_re + z_im * z_im > 4.0) {
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

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.width || y >= params.height) {
        return;
    }

    let cx = params.center_x + (f32(x) - f32(params.width) / 2.0) * params.scale / f32(params.width);
    let cy = params.center_y + (f32(y) - f32(params.height) / 2.0) * params.scale / f32(params.height);

    let iter = mandelbrot(cx, cy, params.max_iter);

    var color: u32;
    if (iter == params.max_iter) {
        color = 0xFF000000u;  // black ARGB
    } else {
        let c = u32(f32(255u) * f32(iter) / f32(params.max_iter));
        color = (0xFFu << 24u) | (c << 16u) | (0u << 8u) | (255u - c); // ARGB gradient
    }

    let index = y * params.bytes_per_row_pixels + x;
    output[index] = color;
}
