// Hyrulor Engine — post-procesado (Fase 8): Bloom / Glow + Composite.
//
// El triángulo a pantalla completa se define en NDC ([-1, 1]) y la UV se
// interpola desde el vertex shader. No se usa `@builtin(position)` del
// fragment shader porque allí son coordenadas de framebuffer en píxeles.

struct VOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var bloom_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> VOut {
    var out: VOut;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    // NDC (y arriba) -> UV de textura (v = 0 arriba).
    out.uv = vec2<f32>((pos.x + 1.0) * 0.5, (1.0 - pos.y) * 0.5);
    return out;
}

@fragment
fn fs_bloom(input: VOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, samp, input.uv);
    let luma = dot(scene.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let excess = max(luma - 0.85, 0.0);
    let bloom_k = pow(excess, 2.0);
    let glow = scene * (bloom_k * 0.8);
    return vec4<f32>(glow.rgb, scene.a);
}

@fragment
fn fs_composite(input: VOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, samp, input.uv);
    let bloom = textureSample(bloom_tex, samp, input.uv);
    let texel = 0.003;
    let blurred = scene
        + textureSample(scene_tex, samp, input.uv + vec2<f32>(texel, texel))
        + textureSample(scene_tex, samp, input.uv + vec2<f32>(-texel, texel))
        + textureSample(scene_tex, samp, input.uv + vec2<f32>(texel, -texel))
        + textureSample(scene_tex, samp, input.uv + vec2<f32>(-texel, -texel));
    let background = blurred / 5.0;
    return vec4<f32>(background.rgb + bloom.rgb, scene.a);
}
