#import bevy_pbr::pbr_functions



struct PbrMaterial {
    color: vec4<f32>,
    emission_factor: vec4<f32>,
    metallic: f32,
    roughness: f32,
    alpha_cutoff: f32,
    flags: u32,
};
@group(2) @binding(0)
var<uniform> material: PbrMaterial;
@group(2) @binding(1)
var diffuse_texture: texture_2d<f32>;
@group(2) @binding(2)
var diffuse_sampler: sampler;

#ifdef PREPASS_FRAGMENT
fn fragment(
    in: prepass_io::VertexOutput,
    @builtin(front_facing) is_front: bool,
) {
    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
#ifdef VISIBILITY_RANGE_DITHER
    pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
#endif  // VISIBILITY_RANGE_DITHER
    alpha_discard(in,material.flags);
}
#else 
@fragment
fn fragment(in: prepass_io::VertexOutput) {
    alpha_discard(in,material.flags);
}
#endif // PREPASS_FRAGMENT

fn alpha_discard(in: prepass_io::VertexOutput, mat: PbrMaterial) {
    var color = mat.color;
    #ifdef VERTEX_COLORS
    color *= in.color;
    #endif
    if (mat.flags & 4u) != 0u {
        #ifdef VERTEX_UVS_A
        color *= textureSample(diffuse_texture, diffuse_sampler, uv);
        #endif
    }
    if (flags & (1 << 0)) != 0 {
        if color.a < mat.alpha_cutoff {
            discard;
        }
    } else if (flags & (1 << 7)) != 0 {
        if color.a < 0.05 {
            discard;
        }
    } else if (flags & (1 << 8)) != 0 {
        if all(color < vec4(0.05)) {
            discard;
        }
    }
}
