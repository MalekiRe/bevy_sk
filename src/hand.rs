use bevy::math::{Quat, Vec3};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, MeshAabb};
use bevy::render::primitives::Aabb;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;

use bevy_mod_xr::hands::{HandBone, XrHandBoneEntities, XrHandBoneRadius, HAND_JOINT_COUNT};
use bevy_mod_xr::session::XrSessionCreatedEvent;
use bevy_mod_xr::spaces::XrSpaceLocationFlags;
use std::f32::consts::{PI, SQRT_2};

const RING_COUNT: usize = SINCOS_ANGLES.len();

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum Finger {
    Thumb = 0,
    Index = 1,
    Middle = 2,
    Ring = 3,
    Little = 4,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum FingerJoint {
    Metacarpal = 0,
    Proximial = 1,
    Intermediate = 2,
    Distal = 3,
    Tip = 4,
}
impl FingerJoint {
    const ALL: [Self; 5] = [
        Self::Metacarpal,
        Self::Proximial,
        Self::Intermediate,
        Self::Distal,
        Self::Tip,
    ];
    const NUM: usize = Self::ALL.len();
    const fn previous_in_chain(&self) -> FingerJoint {
        match self {
            FingerJoint::Metacarpal => FingerJoint::Metacarpal,
            FingerJoint::Proximial => FingerJoint::Metacarpal,
            FingerJoint::Intermediate => FingerJoint::Proximial,
            FingerJoint::Distal => FingerJoint::Intermediate,
            FingerJoint::Tip => FingerJoint::Distal,
        }
    }
}

impl Finger {
    const ALL: [Finger; 5] = [
        Finger::Thumb,
        Finger::Index,
        Finger::Middle,
        Finger::Ring,
        Finger::Little,
    ];
    const NUM: usize = Finger::ALL.len();
    const fn hand_bone(&self, joint: &FingerJoint) -> HandBone {
        match (self, joint) {
            (Finger::Thumb, FingerJoint::Metacarpal) => HandBone::Wrist,
            (Finger::Thumb, FingerJoint::Proximial) => HandBone::ThumbMetacarpal,
            (Finger::Thumb, FingerJoint::Intermediate) => HandBone::ThumbProximal,
            (Finger::Thumb, FingerJoint::Distal) => HandBone::ThumbDistal,
            (Finger::Thumb, FingerJoint::Tip) => HandBone::ThumbTip,
            (finger, joint) => {
                let joint_id = (*finger as u8 * 5) + (*joint as u8) + 1;
                HandBone::get_all_bones()[joint_id as usize]
            }
        }
    }
}

#[derive(Clone, Copy)]
struct HandJoint {
    position: Vec3,
    orientation: Quat,
    radius: f32,
}

const SINCOS_ANGLES: [f32; 7] = [162.0, 90.0, 18.0, 18.0, 306.0, 234.0, 162.0];
const SINCOS_NORM_ANGLES: [f32; 7] = [126.0, 90.0, 54.0, 18.0, 306.0, 234.0, 162.0];

#[derive(Clone, Copy, Component, Debug)]
pub struct DisplayHandMesh;

struct SkHandFinger(Finger);

impl SkHandFinger {
    const fn vertex_count() -> usize {
        let base_verts = RING_COUNT * FingerJoint::NUM;
        let extra_tips = RING_COUNT;
        let end_extra = 1;
        base_verts + extra_tips + end_extra
    }
    const fn start_vert(&self, index: usize) -> usize {
        index * Self::vertex_count()
    }
    const fn end_vert(&self, index: usize) -> usize {
        self.start_vert(index) + Self::vertex_count() - 1
    }
    fn indices(&self, index: usize) -> Vec<u16> {
        let end_vert = self.end_vert(index) as u16 - 7;
        let start_vert = self.start_vert(index) as u16;
        let mut indices = Vec::new();

        // Start cap indices
        indices.extend_from_slice(&[
            (start_vert + 2),
            (start_vert + 1),
            (start_vert + 0),
            (start_vert + 4),
            (start_vert + 3),
            (start_vert + 6),
            (start_vert + 5),
            (start_vert + 4),
            (start_vert + 6),
        ]);

        // Tube faces indices
        for joint in 0..FingerJoint::NUM as u16 {
            for ring in 0..RING_COUNT as u16 {
                // huh?
                // if ring == 2 {
                //     continue;
                // }
                let curr1 = start_vert + (joint * RING_COUNT as u16 + ring);
                let next1 = start_vert + (joint + 1) * RING_COUNT as u16 + ring;
                let curr2 = start_vert + (joint * RING_COUNT as u16 + (ring + 1));
                let next2 = start_vert + (joint + 1) * RING_COUNT as u16 + (ring + 1);

                if ring == RING_COUNT as u16 - 1 {
                    continue;
                }
                indices.extend_from_slice(&[next2, next1, curr1, curr2, next2, curr1]);
            }
        }

        // End cap indices
        indices.extend_from_slice(&[
            (end_vert + 0),
            (end_vert + 1),
            (end_vert + 7),
            (end_vert + 1),
            (end_vert + 2),
            (end_vert + 7),
            (end_vert + 3),
            (end_vert + 4),
            (end_vert + 7),
            (end_vert + 4),
            (end_vert + 5),
            (end_vert + 7),
            (end_vert + 5),
            (end_vert + 6),
            (end_vert + 7),
        ]);

        indices
    }

    fn gen_vertex_colors(&self) -> Vec<[f32; 4]> {
        fn get_color(joint: FingerJoint) -> [f32; 4] {
            let factor = (joint as usize as f32) / (FingerJoint::NUM as f32 - 1.0);
            get_gradient_color(factor).map(|v| v as f32 / u8::MAX as f32)
        }
        let mut colors = Vec::new();
        for joint in FingerJoint::ALL {
            let color = get_color(joint);
            // Push colors for each vertex
            for _v in 0..RING_COUNT {
                colors.push(color);
            }
            if matches!(joint, FingerJoint::Tip) {
                for _v in 0..RING_COUNT {
                    colors.push(color);
                }
            }
        }
        // Extra vertex color
        colors.push(get_color(FingerJoint::Tip));
        colors
    }

    fn gen_vertex_positions_and_normals(
        &self,
        data: &[HandJoint; HAND_JOINT_COUNT],
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let (sincos, sincos_norm) = gen_sincos_and_sincos_norm();
        let tip = data[self.0.hand_bone(&FingerJoint::Tip) as usize];
        let tip_fwd = tip.orientation * -Vec3::Z;
        let tip_up = tip.orientation * Vec3::Y;
        for joint in FingerJoint::ALL {
            let pose_prev = data[self.0.hand_bone(&joint.previous_in_chain()) as usize];
            let pose = data[self.0.hand_bone(&joint) as usize];
            // what? why?
            let orientation = pose_prev.orientation.slerp(pose.orientation, 0.5);

            // Scaling offset to preserve volume
            let mut skew_scale = 1.0;
            if (!matches!(
                joint,
                FingerJoint::Tip | FingerJoint::Distal | FingerJoint::Metacarpal
            )) && ((!matches!(self.0, Finger::Thumb))
                || !matches!(joint, FingerJoint::Metacarpal | FingerJoint::Proximial))
            {
                let fwd_a = pose_prev.orientation * -Vec3::Z;
                let fwd_b = pose.orientation * -Vec3::Z;
                let dot = fwd_a.dot(fwd_b).min(1.0);
                let angle = f32::min(PI / 2.5, dot.acos() / 2.0);
                skew_scale = 1.0 / angle.cos();
            }

            // Local axes
            let right = orientation * Vec3::X;
            let up = orientation * Vec3::new(0.0, skew_scale, 0.0);

            // Scale adjustment
            let mut scale = pose.radius;
            if matches!(self.0, Finger::Thumb)
                && matches!(joint, FingerJoint::Metacarpal | FingerJoint::Proximial)
            {
                scale *= 0.5;
            }

            // Create ring of vertices
            for i in 0..RING_COUNT {
                let norm = (up * sincos_norm[i].y + right * sincos_norm[i].x) * SQRT_2;
                let pos = pose.position + (up * sincos[i].y + right * sincos[i].x) * scale;

                positions.push([pos.x, pos.y, pos.z]);
                normals.push([norm.x, norm.y, norm.z]);
            }

            // Blunt the fingertip
            if matches!(joint, FingerJoint::Tip) {
                scale *= 0.75;
                for i in 0..RING_COUNT {
                    let at = pose.position + tip_fwd * pose.radius * 0.65;
                    let norm = (up * sincos_norm[i].y + right * sincos_norm[i].x) * SQRT_2;
                    let pos = at
                        + (up * sincos[i].y + right * sincos[i].x) * scale
                        + tip_up * pose.radius * 0.25;

                    positions.push([pos.x, pos.y, pos.z]);
                    normals.push([norm.x, norm.y, norm.z]);
                }
            }
        }

        let norm = tip_fwd;
        let pos = tip.position + norm * tip.radius + tip_up * tip.radius * 0.9;

        positions.push([pos.x, pos.y, pos.z]);
        normals.push([norm.x, norm.y, norm.z]);
        (positions, normals)
    }
}

// Function to generate sincos and sincos_norm arrays
fn gen_sincos_and_sincos_norm() -> (
    [Vec3; SINCOS_ANGLES.len()],
    [Vec3; SINCOS_NORM_ANGLES.len()],
) {
    (
        SINCOS_ANGLES.map(|a| Vec3::new(a.to_radians().cos(), a.to_radians().sin(), 0f32)),
        SINCOS_NORM_ANGLES.map(|a| Vec3::new(a.to_radians().cos(), a.to_radians().sin(), 0f32)),
    )
}

fn setup_hand_mesh(
    hands: Query<Entity, With<XrHandBoneEntities>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for e in &hands {
        info!("creating hand");
        let mut hand_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        hand_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0,0.0,0.0]]);
        commands.entity(e).insert((
            Mesh3d(meshes.add(hand_mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Aabb::default(),
        ));
    }
}

const fn get_gradient_color(t: f32) -> [u8; 4] {
    if t < 0.4 {
        let factor = t / 0.4;
        lerp_color([102, 102, 102, 0], [153, 153, 153, 0], factor)
    } else if t < 0.55 {
        let factor = (t - 0.4) / 0.15;
        lerp_color([153, 153, 153, 0], [204, 204, 204, 255], factor)
    } else {
        let factor = (t - 0.55) / 0.45;
        lerp_color([204, 204, 204, 255], [255, 255, 255, 255], factor)
    }
}

const fn lerp_color(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        (a[3] as f32 + (b[3] as f32 - a[3] as f32) * t) as u8,
    ]
}

pub struct HandPlugin;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Last,
            setup_hand_mesh.run_if(on_event::<XrSessionCreatedEvent>),
        );
        app.add_systems(Update, update_hand_mesh);
    }
}

fn update_hand_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    mut hand_mesh: Query<(&Mesh3d, &mut Aabb, &XrHandBoneEntities)>,
    joint_query: Query<(&GlobalTransform, &XrHandBoneRadius, &XrSpaceLocationFlags)>,
) {
    for (mesh_handle, mut aabb, entities) in hand_mesh.iter_mut() {
        let Ok(entities) = joint_query.get_many(entities.0) else {
            warn!("Invalid Hand Joint Entities!");
            continue;
        };
        let data = entities.map(|(transform, radius, _)| {
            let (_, orientation, position) = transform.to_scale_rotation_translation();
            HandJoint {
                position,
                orientation,
                radius: radius.0,
            }
        });
        let vert_count = (RING_COUNT * FingerJoint::NUM + 1) * Finger::NUM;
        let mut positions = Vec::with_capacity(vert_count);
        let mut normals = Vec::with_capacity(vert_count);
        let mut colors = Vec::with_capacity(vert_count);
        let mut indices = Vec::new();

        let mut i = 0;
        let mut fingers = Finger::ALL;
        fingers.reverse();
        for finger in fingers {
            let (_, _, flag) = entities[finger.hand_bone(&FingerJoint::Tip) as usize];
            if (!flag.position_tracked) || (!flag.rotation_tracked) {
                continue;
            }
            let f = SkHandFinger(finger);
            // Doesn't technically need to be re-generated every frame
            indices.extend(f.indices(i));
            colors.extend(f.gen_vertex_colors());

            // This does need to be re-generated every frame
            let (poses, norms) = f.gen_vertex_positions_and_normals(&data);
            positions.extend(poses);
            normals.extend(norms);
            i += 1;
        }
        if positions.is_empty() {
            continue;
        }
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        mesh.insert_indices(Indices::U16(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        let bb = mesh.compute_aabb();
        meshes.insert(mesh_handle, mesh);

        if let Some(bb) = bb {
            *aabb = bb
        }
    }
}
