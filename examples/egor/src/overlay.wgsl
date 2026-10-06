@group(0) @binding(0) var overlay: texture_2d<f32>;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
    return vec4(positions[index], 0., 1.);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = textureLoad(overlay, vec2<i32>(position.xy), 0);
    let srgb = pixel.rgb / max(pixel.a, 0.00001);
    let linear = select(pow((srgb + 0.055) / 1.055, vec3(2.4)), srgb / 12.92, srgb <= vec3(0.04045));
    return vec4(linear, pixel.a);
}
