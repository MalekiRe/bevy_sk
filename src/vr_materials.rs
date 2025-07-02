use std::hash::Hash;

use crate::skytext::SPHERICAL_HARMONICS_HANDLE;
use bevy::asset::{load_internal_asset, weak_handle};
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
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
        load_internal_asset!(
            app,
            SHADER_HANDLE,
            "../assets/pbr_material.wgsl",
            Shader::from_wgsl
        );
        // load_internal_asset!(
        //     app,
        //     SHADER_PREPASS_HANDLE,
        //     "../assets/pbr_material_prepass.wgsl",
        //     Shader::from_wgsl
        // );
        app.add_plugins(MaterialPlugin::<PbrMaterial>::default());
        app.register_type::<PbrMaterial>();
        if self.replace_standard_material {
            app.init_resource::<HandleMapping>();
            app.register_required_components::<MeshMaterial3d<StandardMaterial>, MaterialSwapped>();
        }
    }
}

#[derive(SystemSet, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct MaterialSwapSet;
#[derive(Component, Default)]
#[component(on_add = on_swap_add)]
struct MaterialSwapped;
fn on_swap_add(mut world: DeferredWorld, ctx: HookContext) {
    let Some(id) = world
        .entity(ctx.entity)
        .get::<MeshMaterial3d<StandardMaterial>>()
        .map(|v| v.id())
    else {
        return;
    };
    world
        .commands()
        .entity(ctx.entity)
        .remove::<MeshMaterial3d<StandardMaterial>>();
    let has_handle = world.resource_mut::<HandleMapping>().0.contains_key(&id);
    let handle = if !has_handle {
        let Some(std_mat) = world.resource::<Assets<StandardMaterial>>().get(id) else {
            return;
        };
        let pbr_mat = PbrMaterial::from(std_mat);
        let handle = world.resource_mut::<Assets<PbrMaterial>>().add(pbr_mat);
        world
            .resource_mut::<HandleMapping>()
            .0
            .insert(id, handle.clone());
        handle
    } else {
        world
            .resource::<HandleMapping>()
            .0
            .get(&id)
            .unwrap()
            .clone()
    };
    world
        .commands()
        .entity(ctx.entity)
        .insert(MeshMaterial3d(handle));
}

#[derive(Resource, Default)]
struct HandleMapping(HashMap<AssetId<StandardMaterial>, Handle<PbrMaterial>>);
pub const SHADER_HANDLE: Handle<Shader> = weak_handle!("c0042819-def7-4e25-bb61-62900fbab385");
pub const SHADER_PREPASS_HANDLE: Handle<Shader> =
    weak_handle!("6612069e-ef8e-4367-bc73-ad0af6a47521");

#[derive(Asset, AsBindGroup, PartialEq, Debug, Clone, Reflect)]
#[bind_group_data(PbrMaterialKey)]
// #[bindless(index_table(range(1..13)))]
#[uniform(0, PbrMaterialUniform/* , binding_array(10) */)]
pub struct PbrMaterial {
    pub color: Color,
    pub emission_factor: Color,
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_mode: AlphaMode,

    #[texture(1)]
    #[sampler(2)]
    #[dependency]
    pub diffuse_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    #[dependency]
    pub emission_texture: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    #[dependency]
    pub metal_texture: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    #[dependency]
    pub occlusion_texture: Option<Handle<Image>>,
    #[storage(9, read_only, binding_array(11))]
    #[dependency]
    pub spherical_harmonics: Handle<ShaderStorageBuffer>,
}
impl From<Color> for PbrMaterial {
    fn from(color: Color) -> Self {
        PbrMaterial {
            color,
            ..Default::default()
        }
    }
}
impl From<&StandardMaterial> for PbrMaterial {
    fn from(m: &StandardMaterial) -> Self {
        PbrMaterial {
            color: m.base_color,
            emission_factor: m.emissive.into(),
            metallic: m.metallic,
            roughness: m.perceptual_roughness,
            alpha_mode: m.alpha_mode,
            diffuse_texture: m.base_color_texture.clone(),
            emission_texture: m.emissive_texture.clone(),
            metal_texture: m.metallic_roughness_texture.clone(),
            occlusion_texture: m.occlusion_texture.clone(),
            spherical_harmonics: SPHERICAL_HARMONICS_HANDLE,
        }
    }
}

#[derive(Clone, Default, ShaderType)]
pub struct PbrMaterialUniform {
    pub color: Vec4,
    pub emission_factor: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub alpha_cutoff: f32,
}

impl AsBindGroupShaderType<PbrMaterialUniform> for PbrMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> PbrMaterialUniform {

        PbrMaterialUniform {
            color: self.color.to_linear().to_f32_array().into(),
            emission_factor: self.emission_factor.to_linear().to_f32_array().into(),
            metallic: self.metallic,
            roughness: self.roughness,
            alpha_cutoff: if let AlphaMode::Mask(v) = self.alpha_mode {
                v
            } else {
                1.0
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PbrMaterialKey {
    alpha_mode: HashableAlphaMode,
    diffuse_texture: bool,
    emission_texture: bool,
    metal_texture: bool,
    occlusion_texture: bool,
}
#[derive(Clone, PartialEq, Eq, Copy, Deref, DerefMut, Debug)]
pub struct HashableAlphaMode(pub AlphaMode);
impl Hash for HashableAlphaMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.0 {
            AlphaMode::Opaque => state.write_u8(0),
            AlphaMode::Mask(v) => {
                state.write_u8(1);
                state.write_u32(v.to_bits());
            }
            AlphaMode::Blend => state.write_u8(2),
            AlphaMode::Premultiplied => state.write_u8(3),
            AlphaMode::AlphaToCoverage => state.write_u8(4),
            AlphaMode::Add => state.write_u8(5),
            AlphaMode::Multiply => state.write_u8(6),
        }
    }
}
impl From<AlphaMode> for HashableAlphaMode {
    fn from(value: AlphaMode) -> Self {
        Self(value)
    }
}
impl From<HashableAlphaMode> for AlphaMode {
    fn from(value: HashableAlphaMode) -> Self {
        value.0
    }
}
impl From<&PbrMaterial> for PbrMaterialKey {
    fn from(material: &PbrMaterial) -> Self {
        PbrMaterialKey {
            alpha_mode: material.alpha_mode.into(),
            diffuse_texture: material.diffuse_texture.is_some(),
            emission_texture: material.emission_texture.is_some(),
            metal_texture: material.metal_texture.is_some(),
            occlusion_texture: material.occlusion_texture.is_some(),
        }
    }
}

impl Material for PbrMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HANDLE.into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        SHADER_PREPASS_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    #[allow(unused_variables)]
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let fragment = descriptor.fragment.as_mut().unwrap();
        let data = &key.bind_group_data;
        #[cfg(feature = "stochastic_alpha")]
        {
            fragment.shader_defs.push("STOCHASTIC_ALPHA".into());
        }
        #[cfg(not(feature = "stochastic_alpha"))]
        match data.alpha_mode.0 {
            AlphaMode::Opaque => {}
            AlphaMode::Mask(_) => {
                fragment.shader_defs.push("ALPHA_MASK".into());
            }
            AlphaMode::Blend | AlphaMode::AlphaToCoverage | AlphaMode::Add => {
                fragment.shader_defs.push("ALPHA_CUTOFF".into());
            }
            AlphaMode::Multiply | AlphaMode::Premultiplied => {
                fragment.shader_defs.push("ALPHA_CUTOFF_FULL".into());
            }
        }
        if data.diffuse_texture {
            fragment.shader_defs.push("HAS_DIFFUSE_TEX".into());
        }
        if data.emission_texture {
            fragment.shader_defs.push("HAS_EMISSION_TEX".into());
        }
        if data.metal_texture {
            fragment.shader_defs.push("HAS_METAL_TEX".into());
        }
        if data.occlusion_texture {
            fragment.shader_defs.push("HAS_OCCLUSION_TEX".into());
        }
        Ok(())
    }
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            emission_factor: Color::BLACK,
            metallic: 0.0,
            roughness: 0.0,
            alpha_mode: AlphaMode::Opaque,
            diffuse_texture: None,
            emission_texture: None,
            metal_texture: None,
            occlusion_texture: None,
            spherical_harmonics: SPHERICAL_HARMONICS_HANDLE,
        }
    }
}
