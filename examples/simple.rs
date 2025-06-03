//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::prelude::*;
use bevy_mod_openxr::exts::OxrExtensions;
use bevy_mod_openxr::session::OxrSession;
use bevy_mod_openxr::{add_xr_plugins, init::OxrInitPlugin};
use bevy_mod_xr::session::{XrSessionCreated, XrSessionCreatedEvent};
use bevy_sk::hand::HandPlugin;
use bevy_sk::skytext::{SkytexPlugin, SphericalHarmonicsPlugin};
use bevy_sk::vr_materials::{PbrMaterial, SkMaterialPlugin};

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
        .add_plugins(SkytexPlugin)
        .add_plugins(SphericalHarmonicsPlugin)
        .add_plugins(HandPlugin)
        .add_plugins(SkMaterialPlugin {
            replace_standard_material: false,
        })
        .add_systems(XrSessionCreated, set_requested_refresh_rate)
        .add_systems(Startup, setup_2)
        .add_systems(Startup, set_msaa)
        .add_systems(
            PostUpdate,
            set_msaa.run_if(on_event::<XrSessionCreatedEvent>),
        )
        .insert_resource(AmbientLight {
            color: Default::default(),
            brightness: 500.0,
            affects_lightmapped_meshes: true,
        })
        .run();
}

fn set_msaa(query: Query<Entity, With<Camera>>, mut cmds: Commands) {
    for e in &query {
        cmds.entity(e).insert(Msaa::Sample4);
    }
}

fn set_requested_refresh_rate(session: ResMut<OxrSession>) {
    if let Err(err) = session.request_display_refresh_rate(120.0) {
        error!("errror while requesting refresh rate: {err}");
    }
}
fn setup_2(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PbrMaterial>>,
) {
    let mut white = PbrMaterial::from(Color::WHITE); // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(white)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    let mut cube_mat: PbrMaterial = Color::srgb_u8(124, 144, 255).into();
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 0.1))),
        MeshMaterial3d(materials.add(cube_mat)),
        Transform::from_xyz(0.0, 0.7, 0.0),
    ));
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
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(white)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    let mut cube_mat: StandardMaterial = Color::srgb_u8(124, 144, 255).into();
    cube_mat.unlit = false;
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 0.1))),
        MeshMaterial3d(materials.add(cube_mat)),
        Transform::from_xyz(0.0, 0.7, 0.0),
    ));
}
