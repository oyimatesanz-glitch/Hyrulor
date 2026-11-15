// Hyrulor Engine — shader del orbe holográfico.
//
// El uniforme se declara como 8 vec4 (128 bytes) para respetar la
// alineación del address space Uniform (stride múltiplo de 16). El layout
// de bytes coincide con el `[f32; 32]` que escribe la CPU:
//   u[0..4]  : matriz MVP (column-major, 4 columnas)
//   u[4].x   : tiempo en segundos
//   u[4].yzw : color base RGB
//   u[5].xyz : posición de la cámara
//   u[5].w   : velocidad del pulso
//   u[6].x   : exponente de fresnel
//   u[6].y   : brillo global

@group(0) @binding(0) var<uniform> u: array<vec4<f32>, 8>;

fn load_mvp() -> mat4x4<f32> {
    return mat4x4<f32>(u[0], u[1], u[2], u[3]);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = load_mvp() * vec4<f32>(position, 1.0);
    out.world_pos = position;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = u[4].x;
    let base_color = u[4].yzw;
    let camera = u[5].xyz;
    let pulse_speed = u[5].w;
    let fresnel_power = u[6].x;
    let brightness = u[6].y;

    // 1. Escaneo holográfico: una línea horizontal que sube y baja.
    let scan_y = sin(time * 2.0) * 1.5;
    let distance_to_scan = abs(in.world_pos.y - scan_y);
    let scanline = 1.0 - smoothstep(0.0, 0.2, distance_to_scan);

    // 2. Pulso de respiración, modulado por el estado.
    let pulse = 0.85 + 0.15 * sin(time * pulse_speed);

    // 3. Fresnel: los bordes brillan más (efecto cristal holográfico).
    //    Para geometría centrada en el origen, `normalize(world_pos)` es
    //    una normal aproximada suficiente.
    let view_dir = normalize(camera - in.world_pos);
    let ndv = abs(dot(normalize(in.world_pos), view_dir));
    let fresnel = pow(1.0 - ndv, fresnel_power);

    // 4. Composición: núcleo + borde + resplandor del escáner.
    let core = base_color * pulse * brightness;
    let rim = base_color * fresnel * 1.6;
    let scan_glow = base_color * scanline * 2.0;

    let final_color = core + rim + scan_glow;
    return vec4<f32>(final_color, 1.0);
}
