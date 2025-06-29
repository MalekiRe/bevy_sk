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
    #[storage(9, read_only, binding_array(11))]
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
            double_sided: m.double_sided,
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
    pub flags: u32,
}

impl AsBindGroupShaderType<PbrMaterialUniform> for PbrMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> PbrMaterialUniform {
        let flags = PbrMaterialFlags::from(self);

        PbrMaterialUniform {
            color: self.color.to_linear().to_f32_array().into(),
            emission_factor: self.emission_factor.to_linear().to_f32_array().into(),
            metallic: self.metallic,
            roughness: self.roughness,
            flags: flags.bits(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PbrMaterialKey(PbrMaterialFlags);
impl From<&PbrMaterial> for PbrMaterialFlags {
    fn from(value: &PbrMaterial) -> Self {
        let mut flags = PbrMaterialFlags::empty();

        if value.diffuse_texture.is_some() {
            flags |= PbrMaterialFlags::DIFFUSE_TEXTURE;
        }
        if value.emission_texture.is_some() {
            flags |= PbrMaterialFlags::EMISSION_TEXTURE;
        }
        if value.metal_texture.is_some() {
            flags |= PbrMaterialFlags::METAL_TEXTURE;
        }
        if value.occlusion_texture.is_some() {
            flags |= PbrMaterialFlags::OCCLUSION_TEXTURE;
        }
        if value.double_sided {
            flags |= PbrMaterialFlags::DOUBLE_SIDED;
        }

        match value.alpha_mode {
            AlphaMode::Opaque => flags |= PbrMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(_) => flags |= PbrMaterialFlags::ALPHA_MODE_MASK,
            _ => {}
        };
        flags
    }
}
impl From<&PbrMaterial> for PbrMaterialKey {
    fn from(material: &PbrMaterial) -> Self {
        PbrMaterialKey(PbrMaterialFlags::from(material))
    }
}

impl Material for PbrMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, PartialEq, Eq, Hash, Copy)]
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
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            diffuse_texture: None,
            emission_texture: None,
            metal_texture: None,
            occlusion_texture: None,
            spherical_harmonics: SPHERICAL_HARMONICS_HANDLE,
        }
    }
}
