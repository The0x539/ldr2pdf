#import bevy_pbr::{
    pbr_bindings,
    pbr_types,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

#ifdef OIT_ENABLED
    #import bevy_core_pipeline::oit::oit_draw
#endif

fn color_overlay(
    base: vec4<f32>,
    vert: vec4<f32>,
) -> vec4<f32> {
    return vec4(mix(base.rgb, vert.rgb, vert.a), base.a);
}

@fragment
fn fragment(
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var in = vertex_output;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // apply my color customization
    #ifdef VERTEX_COLORS
        pbr_input.material.base_color = color_overlay(pbr_bindings::material.base_color, in.color);
    #endif

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    #ifdef OIT_ENABLED
        let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
        if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
            // The fragments will only be drawn during the oit resolve pass.
            oit_draw(in.position, out.color);
            discard;
        }
    #endif

    return out;
}
