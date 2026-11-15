//! Estado visual del orbe y geometría del holograma.
//!
//! El orbe no es decorativo: su color, pulso y brillo codifican el estado
//! real del sistema (ver `OrbState`). La geometría (esfera + anillo orbital)
//! se genera proceduralmente y se dibuja con `wgpu`.

/// Estado cognitivo del orbe, espejo del ciclo de vida de una petición.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbState {
    /// Sin petición en curso: brillo bajo, movimiento lento.
    Idle,
    /// Petición en vuelo: pulso dorado acelerado.
    Thinking,
    /// El daemon exige autorización humana: rojo suave, escudo.
    Blocked,
    /// Fallo (daemon o protocolo): rojo apagado.
    Error,
    /// Daemon inaccesible: gris tenue.
    Offline,
}

impl OrbState {
    /// Etiqueta legible para la UI.
    pub fn label(self) -> &'static str {
        match self {
            OrbState::Idle => "En línea",
            OrbState::Thinking => "Pensando…",
            OrbState::Blocked => "Autorización requerida",
            OrbState::Error => "Error",
            OrbState::Offline => "Sin conexión",
        }
    }

    /// Color base (RGB) del orbe para este estado.
    pub fn color(self) -> [f32; 3] {
        match self {
            OrbState::Idle => [0.83, 0.69, 0.22],    // oro Hyrulor
            OrbState::Thinking => [1.0, 0.84, 0.30], // oro brillante
            OrbState::Blocked => [0.85, 0.27, 0.22], // rojo suave
            OrbState::Error => [0.90, 0.16, 0.13],   // rojo apagado
            OrbState::Offline => [0.42, 0.42, 0.45], // gris
        }
    }

    /// Velocidad del pulso de respiración (multiplicador del seno).
    pub fn pulse_speed(self) -> f32 {
        match self {
            OrbState::Idle => 1.4,
            OrbState::Thinking => 4.0,
            OrbState::Blocked => 0.8,
            OrbState::Error => 7.0,
            OrbState::Offline => 0.25,
        }
    }

    /// Exponente del fresnel: mayor -> borde más fino.
    pub fn fresnel_power(self) -> f32 {
        match self {
            OrbState::Idle => 2.0,
            OrbState::Thinking => 3.0,
            OrbState::Blocked => 2.5,
            OrbState::Error => 2.0,
            OrbState::Offline => 1.5,
        }
    }

    /// Brillo global.
    pub fn brightness(self) -> f32 {
        match self {
            OrbState::Idle => 1.0,
            OrbState::Thinking => 1.25,
            OrbState::Blocked => 1.0,
            OrbState::Error => 1.1,
            OrbState::Offline => 0.6,
        }
    }
}

/// Vértice del holograma: posición + color (usado como tinte neutro).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Esfera UV centrada en el origen. El color es blanco: el estado del orbe
/// se aplica vía uniform, no en los vértices.
pub fn create_sphere(radius: f32, sectors: u32, stacks: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for i in 0..=stacks {
        let stack_angle =
            std::f32::consts::PI / 2.0 - std::f32::consts::PI * (i as f32) / (stacks as f32);
        let xy = radius * stack_angle.cos();
        let z = radius * stack_angle.sin();
        for j in 0..=sectors {
            let sector_angle = 2.0 * std::f32::consts::PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();
            vertices.push(Vertex {
                position: [x, y, z],
                color: [1.0, 1.0, 1.0],
            });
        }
    }
    for i in 0..stacks {
        for j in 0..sectors {
            let first = i * (sectors + 1) + j;
            let second = first + sectors + 1;
            indices.push(first);
            indices.push(first + 1);
            indices.push(first);
            indices.push(second);
        }
    }
    (vertices, indices)
}

/// Toro (anillo orbital) alrededor del origen.
pub fn create_torus(
    radius: f32,
    tube_radius: f32,
    radial_segments: u32,
    tubular_segments: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for j in 0..=tubular_segments {
        let v = j as f32 / tubular_segments as f32 * std::f32::consts::PI * 2.0;
        for i in 0..=radial_segments {
            let u = i as f32 / radial_segments as f32 * std::f32::consts::PI * 2.0;
            let x = (radius + tube_radius * u.cos()) * v.cos();
            let y = (radius + tube_radius * u.cos()) * v.sin();
            let z = tube_radius * u.sin();
            vertices.push(Vertex {
                position: [x, y, z],
                color: [1.0, 1.0, 1.0],
            });
        }
    }
    for j in 1..=tubular_segments {
        for i in 1..=radial_segments {
            let a = (radial_segments + 1) * (j - 1) + (i - 1);
            let b = (radial_segments + 1) * j + (i - 1);
            let c = (radial_segments + 1) * j + i;
            let d = (radial_segments + 1) * (j - 1) + i;
            indices.push(a);
            indices.push(b);
            indices.push(b);
            indices.push(c);
            indices.push(c);
            indices.push(d);
            indices.push(d);
            indices.push(a);
        }
    }
    (vertices, indices)
}
