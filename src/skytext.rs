use crate::vr_materials::{PbrMaterial, SHADER_HANDLE};
use bevy::asset::load_internal_asset;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, ShaderType, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension, UniformBuffer,
};
use bevy::render::storage::ShaderStorageBuffer;
use std::ops::Mul;

pub struct SkytexPlugin;

pub struct SphericalHarmonicsPlugin;

impl Plugin for SphericalHarmonicsPlugin {
    fn build(&self, app: &mut App) {
        app
            //.add_plugins(MaterialPlugin::<PbrMaterial>::default())
            .add_systems(Startup, setup_spherical_harmonics)
            .add_systems(Update, update_spherical_harmonics);
    }
}

pub const SPHERICAL_HARMONICS_HANDLE: Handle<ShaderStorageBuffer> =
    Handle::weak_from_u128(0x4e5e4e9a1e9161808328acd635928dbb);
fn setup_spherical_harmonics(
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let sh_coeffs = DEFAULT_LIGHTING;
    buffers.insert(
        &SPHERICAL_HARMONICS_HANDLE,
        ShaderStorageBuffer::from(sh_coeffs),
    );
}

fn update_spherical_harmonics(mut buffers: ResMut<Assets<ShaderStorageBuffer>>) {
    let mut windowed_lighting = DEFAULT_LIGHTING.clone();
    sh_windowing(&mut windowed_lighting, 1.0);
    if let Some(buffer) = buffers.get_mut(&SPHERICAL_HARMONICS_HANDLE) {
        buffer.set_data(windowed_lighting);
    }
    // Update spherical harmonics coefficients here
    // For example:
    // sh_uniform.buffer.get_mut().coefficients[0] = [0.74, 0.74, 0.73, 1.0];
    // ...
}

impl Plugin for SkytexPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            |mut commands: Commands,
             query: Query<(Entity, &mut Camera3d)>,
             mut images: ResMut<Assets<Image>>| {
                for (e, _c) in query.iter() {
                    let mut windowed_lighting = DEFAULT_LIGHTING.clone();
                    sh_windowing(&mut windowed_lighting, 1.0);
                    commands.entity(e).insert(bevy::core_pipeline::Skybox {
                        image: images
                            .add(generate_cubemap(&windowed_lighting, 16, 0.3f32, 6.0).unwrap()),
                        brightness: 1000.0,
                        rotation: Quat::IDENTITY,
                    });
                }
            },
        );
    }
}

pub const DEFAULT_LIGHTING: SphericalHarmonics = SphericalHarmonics {
    coefficients: [
        Vec3::new(0.74, 0.74, 0.73),
        Vec3::new(0.24, 0.25, 0.26),
        Vec3::new(0.09, 0.09, 0.09),
        Vec3::new(0.05, 0.05, 0.06),
        Vec3::new(-0.01, -0.01, -0.01),
        Vec3::new(-0.03, -0.03, -0.03),
        Vec3::new(0.00, 0.00, 0.00),
        Vec3::new(-0.02, -0.02, -0.02),
        Vec3::new(0.04, 0.04, 0.04),
    ],
};

// tex_gen_cubemap_sh(lighting, 16, 0.3f);
#[derive(ShaderType, Default, Copy, Clone, Debug, PartialEq, Reflect)]
pub struct SphericalHarmonics {
    pub coefficients: [Vec3; 9],
}

pub(crate) fn sh_windowing(harmonics: &mut SphericalHarmonics, window_width: f32) {
    let mut i = 0;
    for band in 0..=2 {
        let s = 1.0 / (1.0 + window_width * (band * band * (band + 1) * (band + 1)) as f32);
        for _ in -band..=band {
            harmonics.coefficients[i] *= s;
            i += 1;
        }
    }
}

fn plane_ray_intersect(plane: (Vec3, f32), ray: (Vec3, Vec3)) -> (bool, Vec3) {
    let (normal, d) = plane;
    let (ray_pos, ray_dir) = ray;

    // Calculate t = -(Pi . N + d) / (V . N)
    let denominator = ray_dir.dot(normal);
    let t = -(ray_pos.dot(normal) + d) / denominator;

    // Calculate the intersection point: Pf = Pi + tV
    let out_pt = ray_pos + ray_dir * t;

    // Return (true, out_pt) if t >= 0, otherwise (false, out_pt)
    (t >= 0.0, out_pt)
}

pub(crate) fn generate_cubemap(
    lookup: &SphericalHarmonics,
    face_size: u32,
    light_spot_size_pct: f32,
    light_spot_intensity: f32,
) -> Option<Image> {
    // Calculate information used to create the light spot
    let light_dir = sh_dominant_dir(lookup);
    let light_col = sh_lookup(lookup, -light_dir) * light_spot_intensity;
    let mut light_pt = Vec3::splat(10000.0);

    for i in 0..6 {
        let p1 = math_cubemap_corner(i * 4);
        let p2 = math_cubemap_corner(i * 4 + 1);
        let p3 = math_cubemap_corner(i * 4 + 2);
        let plane = plane_from_points(p1, p2, p3);
        let (b, pt) = plane_ray_intersect(plane, (Vec3::ZERO, light_dir));
        if !b && pt.length_squared() < light_pt.length_squared() {
            light_pt = pt;
        }
    }

    let size = face_size.next_power_of_two();
    let half_px = 0.5 / size as f32;
    let size2 = (size * size) as usize;

    let mut data = vec![Vec4::ZERO; size2 * 6];

    let size2 = size2 as i32;

    for i in 0..6 {
        let p1 = math_cubemap_corner(i * 4);
        let p2 = math_cubemap_corner(i * 4 + 1);
        let p3 = math_cubemap_corner(i * 4 + 2);
        let p4 = math_cubemap_corner(i * 4 + 3);

        for y in 0..size {
            let mut py = 1.0 - (y as f32 / size as f32 + half_px);
            if i == 2 {
                py = 1.0 - py;
            }
            for x in 0..size {
                let mut px = x as f32 / size as f32 + half_px;
                if i == 2 {
                    px = 1.0 - px;
                }
                let pl = p1.lerp(p4, py);
                let pr = p2.lerp(p3, py);
                let pt = pl.lerp(pr, px);

                // Calculate distance before normalizing pt
                let dist = (pt - light_pt).abs().max_element();

                let pt_normalized = pt.normalize();

                let color = if dist < light_spot_size_pct {
                    light_col
                } else {
                    sh_lookup(lookup, pt_normalized)
                };

                data[(i * size2 + (y as i32 * size as i32 + x as i32)) as usize] = color;
            }
        }
    }

    let image_data: Vec<u8> = data
        .into_iter()
        .flat_map(|v| {
            vec![
                (v.x * 255.0).clamp(0.0, 255.0) as u8,
                (v.y * 255.0).clamp(0.0, 255.0) as u8,
                (v.z * 255.0).clamp(0.0, 255.0) as u8,
                (v.w * 255.0).clamp(0.0, 255.0) as u8,
            ]
        })
        .collect();

    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        image_data,
        TextureFormat::Rgba8Unorm,
        Default::default(),
    );

    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    Some(image)
}

fn sh_dominant_dir(harmonics: &SphericalHarmonics) -> Vec3 {
    let dir = Vec3::new(
        harmonics.coefficients[3].x * 0.3
            + harmonics.coefficients[3].y * 0.59
            + harmonics.coefficients[3].z,
        harmonics.coefficients[1].x * 0.3
            + harmonics.coefficients[1].y * 0.59
            + harmonics.coefficients[1].z,
        harmonics.coefficients[2].x * 0.3
            + harmonics.coefficients[2].y * 0.59
            + harmonics.coefficients[2].z,
    );
    -dir.normalize()
}

fn sh_lookup(harmonics: &SphericalHarmonics, normal: Vec3) -> Vec4 {
    const PI: f32 = std::f32::consts::PI;
    const COSINE_A0: f32 = PI;
    const COSINE_A1: f32 = (2.0 * PI) / 3.0;
    const COSINE_A2: f32 = PI * 0.25;

    let mut result = Vec3::ZERO;

    // Band 0
    result += harmonics.coefficients[0] * (0.282095 * COSINE_A0);
    // Band 1
    result += harmonics.coefficients[1] * (0.488603 * normal.y * COSINE_A1);
    result += harmonics.coefficients[2] * (0.488603 * normal.z * COSINE_A1);
    result += harmonics.coefficients[3] * (0.488603 * normal.x * COSINE_A1);
    // Band 2
    result += harmonics.coefficients[4] * (1.092548 * normal.x * normal.y * COSINE_A2);
    result += harmonics.coefficients[5] * (1.092548 * normal.y * normal.z * COSINE_A2);
    result +=
        harmonics.coefficients[6] * (0.315392 * (3.0 * normal.z * normal.z - 1.0) * COSINE_A2);
    result += harmonics.coefficients[7] * (1.092548 * normal.x * normal.z * COSINE_A2);
    result += harmonics.coefficients[8]
        * (0.546274 * (normal.x * normal.x - normal.y * normal.y) * COSINE_A2);

    Vec4::new(result.x, result.y, result.z, 1.0)
}

fn math_cubemap_corner(i: i32) -> Vec3 {
    let neg = if (i / 4) % 2 == 0 { 1.0 } else { -1.0 };
    let nx = ((i + 24) / 16) % 2;
    let ny = (i / 8) % 2;
    let nz = (i / 16) % 2;
    let u = ((i + 1) / 2) % 2; // U: 0,1,1,0
    let v = (i / 2) % 2; // V: 0,0,1,1

    Vec3::new(
        if nx != 0 {
            neg
        } else if ny != 0 {
            if u != 0 { -1.0 } else { 1.0 }.mul(neg)
        } else {
            if u != 0 { 1.0 } else { -1.0 }.mul(neg)
        },
        if nx != 0 || nz != 0 {
            if v != 0 {
                1.0
            } else {
                -1.0
            }
        } else {
            neg
        },
        if nx != 0 {
            if u != 0 { -1.0 } else { 1.0 }.mul(neg)
        } else if ny != 0 {
            if v != 0 {
                1.0
            } else {
                -1.0
            }
        } else {
            neg
        },
    )
}

fn plane_from_points(p1: Vec3, p2: Vec3, p3: Vec3) -> (Vec3, f32) {
    let normal = (p2 - p1).cross(p3 - p1).normalize();
    let d = -normal.dot(p1);
    (normal, d)
}
