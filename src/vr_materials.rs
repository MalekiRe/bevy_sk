use crate::skytext::{SphericalHarmonics, DEFAULT_LIGHTING, SPHERICAL_HARMONICS_HANDLE};
use bevy::pbr::SHADOW_SAMPLING_HANDLE;
use bevy::render::render_resource::Face;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderRef, ShaderType},
        texture::GpuImage,
    },
};

#[derive(Default)]
pub struct SkMaterialPlugin {
    pub replace_standard_material: bool,
}
impl Plugin for SkMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PbrMaterial>();
        if self.replace_standard_material {
            app.add_systems(PostUpdate, replace_material);
        }
    }
}

pub fn replace_material(
    mut commands: Commands,
    query: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    mut pbr_material: ResMut<Assets<PbrMaterial>>,
    standard_material: Res<Assets<StandardMaterial>>,
) {
    for (e, m) in query.iter() {
        let m = standard_material.get(m).unwrap();
        commands
            .entity(e)
            .insert(MeshMaterial3d(pbr_material.add(PbrMaterial {
            color: m.base_color,
            emission_factor: Default::default(),
            metallic: m.metallic,
            roughness: m.perceptual_roughness,
            tex_scale: 1.0,
            alpha_mode: m.alpha_mode,
            double_sided: m.double_sided,
            spherical_harmonics: SPHERICAL_HARMONICS_HANDLE,
            diffuse_texture: /*m.diffuse_transmission_texture.clone()*/ Default::default(),
            emission_texture: m.emissive_texture.clone(),
            metal_texture: m.metallic_roughness_texture.clone(),
            occlusion_texture: m.occlusion_texture.clone(),
            color_texture: m.base_color_texture.clone(),
        })));
        commands
            .entity(e)
            .remove::<MeshMaterial3d<StandardMaterial>>();
    }
}

pub const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0x2d86c40a165b);

#[derive(Asset, AsBindGroup, PartialEq, Debug, Clone, Reflect)]
/*#[bind_group_data(PbrMaterialKey)]*/
#[uniform(0, PbrMaterialUniform)]
pub struct PbrMaterial {
    pub color: Color,
    pub emission_factor: Color,
    pub metallic: f32,
    pub roughness: f32,
    pub tex_scale: f32,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,

    #[texture(1)]
    #[sampler(2)]
    pub diffuse_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub emission_texture: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub metal_texture: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    pub occlusion_texture: Option<Handle<Image>>,
    #[texture(9)]
    #[sampler(10)]
    pub color_texture: Option<Handle<Image>>,
    #[storage(11, read_only)]
    pub spherical_harmonics: Handle<ShaderStorageBuffer>,
}

#[derive(Clone, Default, ShaderType)]
pub struct PbrMaterialUniform {
    pub color: Vec4,
    pub emission_factor: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub tex_scale: f32,
    pub flags: u32,
}

impl AsBindGroupShaderType<PbrMaterialUniform> for PbrMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> PbrMaterialUniform {
        let mut flags = PbrMaterialFlags::empty();

        if self.diffuse_texture.is_some() {
            flags |= PbrMaterialFlags::DIFFUSE_TEXTURE;
        }
        if self.emission_texture.is_some() {
            flags |= PbrMaterialFlags::EMISSION_TEXTURE;
        }
        if self.metal_texture.is_some() {
            flags |= PbrMaterialFlags::METAL_TEXTURE;
        }
        if self.occlusion_texture.is_some() {
            flags |= PbrMaterialFlags::OCCLUSION_TEXTURE;
        }
        if self.double_sided {
            flags |= PbrMaterialFlags::DOUBLE_SIDED;
        }

        match self.alpha_mode {
            AlphaMode::Opaque => flags |= PbrMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(_) => flags |= PbrMaterialFlags::ALPHA_MODE_MASK,
            _ => {}
        }

        PbrMaterialUniform {
            color: self.color.to_linear().to_f32_array().into(),
            emission_factor: self.emission_factor.to_linear().to_f32_array().into(),
            metallic: self.metallic,
            roughness: self.roughness,
            tex_scale: self.tex_scale,
            flags: flags.bits(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PbrMaterialKey {
    cull_mode: Option<Face>,
    alpha_mode: bool,
}

impl From<&PbrMaterial> for PbrMaterialKey {
    fn from(material: &PbrMaterial) -> Self {
        PbrMaterialKey {
            cull_mode: if material.double_sided {
                None
            } else {
                Some(Face::Back)
            },
            alpha_mode: material.alpha_mode == AlphaMode::Blend,
        }
    }
}

impl Material for PbrMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    /*fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        Ok(())
    }*/
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct PbrMaterialFlags: u32 {
        const ALPHA_MODE_MASK    = (1 << 0);
        const ALPHA_MODE_OPAQUE  = (1 << 1);
        const DIFFUSE_TEXTURE    = (1 << 2);
        const DOUBLE_SIDED       = (1 << 3);
        const EMISSION_TEXTURE   = (1 << 4);
        const METAL_TEXTURE      = (1 << 5);
        const OCCLUSION_TEXTURE  = (1 << 6);
    }
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            emission_factor: Color::BLACK,
            metallic: 0.0,
            roughness: 0.0,
            tex_scale: 1.0,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            spherical_harmonics: SPHERICAL_HARMONICS_HANDLE,
            diffuse_texture: None,
            emission_texture: None,
            metal_texture: None,
            occlusion_texture: None,
            color_texture: None,
        }
    }
}
