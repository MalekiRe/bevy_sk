#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings
#import bevy_pbr::utils
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_vertex_output,
    mesh_view_bindings::view,
}
#import bevy_pbr::mesh_bindings::mesh
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
}

struct PbrMaterial {
    color: vec4<f32>,
    emission_factor: vec4<f32>,
    metallic: f32,
    roughness: f32,
    flags: u32,
};

struct PbrMaterialBindings {
    material: u32,
    diffuse_texture: u32,
    diffuse_texture_sampler: u32,
    emission_texture: u32,
    emission_texture_sampler: u32,
    metal_texture: u32,
    metal_texture_sampler: u32,
    occlusion_texture: u32,
    occlusion_texture_sampler: u32,
    color_texture: u32,
    color_texture_sampler: u32,
    spherical_harmonics: u32,
}

#ifdef BINDLESS
@group(2) @binding(0) var<storage> materials: array<PbrMaterialBindings>;
@group(2) @binding(10) var<storage> material_data: binding_array<PbrMaterial>;
// @group(2) @binding(11) var<storage> spherical_harmonics_buffer: binding_array<array<vec3<f32>, 9>>;
#else
@group(2) @binding(0)
var<uniform> material: PbrMaterial;
@group(2) @binding(1)
var diffuse_texture: texture_2d<f32>;
@group(2) @binding(2)
var diffuse_sampler: sampler;
@group(2) @binding(3)
var emission_texture: texture_2d<f32>;
@group(2) @binding(4)
var emission_sampler: sampler;
@group(2) @binding(5)
var metal_texture: texture_2d<f32>;
@group(2) @binding(6)
var metal_sampler: sampler;
@group(2) @binding(7)
var occlusion_texture: texture_2d<f32>;
@group(2) @binding(8)
var occlusion_sampler: sampler;
@group(2) @binding(9)
var color_texture: texture_2d<f32>;
@group(2) @binding(10) 
var<storage, read> spherical_harmonics_buffer: array<vec3<f32>, 9>;
#endif

// Cubemap is part of the view bindings in Bevy
// @group(0) @binding(10)
// var environment_map: texture_cube<f32>;
// @group(0) @binding(11)
// var environment_sampler: sampler;

/*struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};*/

fn sk_lighting(normal: vec3<f32>, spherical_harmonics: array<vec3<f32>, 9>) -> vec3<f32> {
    // Band 0
    var result = spherical_harmonics[0];

    // Band 1
    result += spherical_harmonics[1] * normal.y;
    result += spherical_harmonics[2] * normal.z;
    result += spherical_harmonics[3] * normal.x;

    // Band 2
    let n = normal * normal;
    let n2 = normal.xyz * normal.yzx;
    result += spherical_harmonics[4] * n2.x;
    result += spherical_harmonics[5] * n2.y;
    result += spherical_harmonics[6] * (3.0 * n.z - 1.0);
    result += spherical_harmonics[7] * n2.z;
    result += spherical_harmonics[8] * (n.x - n.y);

    return result;
}


fn sk_pbr_fresnel_schlick_roughness(ndotv: f32, F0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(1.0 - ndotv, 5.0);
}

fn sk_pbr_brdf_appx(roughness: f32, ndotv: f32) -> vec2<f32> {
    let c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * ndotv)) * r.x + r.y;
    return vec2(-1.04, 1.04) * a004 + r.zw;
}

@fragment
fn fragment(@builtin(front_facing) is_front: bool, in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    let material =  material_data[materials[slot].material];
    
    let diffuse_texture = bindless_textures_2d[materials[slot].diffuse_texture];
    let diffuse_sampler = bindless_samplers_filtering[materials[slot].diffuse_texture_sampler];


    let emission_texture = bindless_textures_2d[materials[slot].emission_texture];
    let emission_sampler = bindless_samplers_filtering[materials[slot].emission_texture_sampler];

    let metal_texture = bindless_textures_2d[materials[slot].metal_texture];
    let metal_sampler = bindless_samplers_filtering[materials[slot].metal_texture_sampler];

    let occlusion_texture = bindless_textures_2d[materials[slot].occlusion_texture];
    let occlusion_sampler = bindless_samplers_filtering[materials[slot].occlusion_texture_sampler];

    let color_texture = bindless_textures_2d[materials[slot].color_texture];
    let color_sampler = bindless_samplers_filtering[materials[slot].color_texture_sampler];

    // let spherical_harmonics_buffer = spherical_harmonics_buffer[materials[slot].spherical_harmonics];
#endif
    let pbr_input = pbr_input_from_vertex_output(in, is_front, false);

    #ifdef VERTEX_UVS_A
    var uv = in.uv;
    if (material.flags & 128u) != 0u {
        uv = vec2(1.0) - uv;
    }
    #endif

    var albedo = material.color;
    #ifdef VERTEX_COLORS
    albedo *= in.color;
    #endif

    if (material.flags & 4u) != 0u {
        #ifdef VERTEX_UVS_A
        albedo *= textureSample(diffuse_texture, diffuse_sampler, uv);
        #endif
    }


    var emissive = material.emission_factor.rgb;
    if (material.flags & 16u) != 0u {
        #ifdef VERTEX_UVS_A
        emissive *= textureSample(emission_texture, emission_sampler, uv).rgb;
        #endif
    }

    var metal_rough = vec2(material.roughness, material.metallic);
    if (material.flags & 32u) != 0u {
        #ifdef VERTEX_UVS_A
        metal_rough *= textureSample(metal_texture, metal_sampler, uv).bg;
        #endif
    }

    var ao = 1.0;
    if (material.flags & 64u) != 0u {
        #ifdef VERTEX_UVS_A
        ao = textureSample(occlusion_texture, occlusion_sampler, uv).r;
        #endif
    }

    let N = normalize(pbr_input.world_normal);
    let V = normalize(view.world_position.xyz - in.world_position.xyz);
    let R = reflect(-V, N);

    let ndotv = max(dot(N, V), 0.0001);
    let F0 = mix(vec3(0.04), albedo.rgb, metal_rough.y);

    let F = sk_pbr_fresnel_schlick_roughness(ndotv, F0, metal_rough.x);
    let kS = F;
    var kD = vec3(1.0) - kS;
    kD *= 1.0 - metal_rough.y;

    // let irradiance = sk_lighting(N, spherical_harmonics_buffer);
    //
    // let diffuse = albedo.rgb * irradiance;
    let diffuse = albedo.rgb;

    let mip = metal_rough.x * f32(view.mip_bias);
    //let prefilteredColor = diffuse;
    //let prefilteredColor = textureSampleLevel(view.environment_map, view.environment_sampler, R, mip).rgb;

    let envBRDF = sk_pbr_brdf_appx(metal_rough.x, ndotv);
    let specular = (F * envBRDF.x + envBRDF.y);

    var color = (kD * diffuse + specular) * ao;
    //color += emissive;

    return vec4(color, albedo.a);
}
