//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::prelude::*;
use bevy_mod_openxr::exts::OxrExtensions;
use bevy_mod_openxr::session::OxrSession;
use bevy_mod_openxr::{add_xr_plugins, init::OxrInitPlugin};
use bevy_mod_xr::session::XrSessionCreated;
use bevy_sk::hand::HandPlugin;
use bevy_sk::skytext::{SkytexPlugin, SphericalHarmonicsPlugin};
use bevy_sk::vr_materials::SkMaterialPlugin;

fn main() {
    App::new()
        .add_plugins(add_xr_plugins(DefaultPlugins).set(OxrInitPlugin {
            exts: {
                let mut exts = OxrExtensions::default();
                exts.fb_display_refresh_rate = true;
                exts.enable_hand_tracking();
                exts
            },
            ..default()
        }))
        // .add_plugins(SkytexPlugin)
        .add_plugins(SphericalHarmonicsPlugin)
        .add_plugins(HandPlugin)
        .add_plugins(SkMaterialPlugin {
            replace_standard_material: true,
        })
        .add_systems(XrSessionCreated, set_requested_refresh_rate)
        .insert_resource(Msaa::Sample4)
        .add_systems(Startup, setup)
        .insert_resource(AmbientLight {
            color: Default::default(),
            brightness: 500.0,
        })
        .run();
}

fn set_requested_refresh_rate(session: ResMut<OxrSession>) {
    if let Err(err) = session.request_display_refresh_rate(120.0) {
        error!("errror while requesting refresh rate: {err}");
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut white: StandardMaterial = Color::WHITE.into();
    white.unlit = true;
    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(white),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    let mut cube_mat: StandardMaterial = Color::srgb_u8(124, 144, 255).into();
    cube_mat.unlit = true;
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.1, 0.1, 0.1)),
        material: materials.add(cube_mat),
        transform: Transform::from_xyz(0.0, 0.7, 0.0),
        ..default()
    });
}
