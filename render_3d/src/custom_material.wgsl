#import bevy_pbr::{
    forward_io,
    forward_io::{Vertex, FragmentOutput},
    mesh_functions,
    pbr_bindings,
    pbr_functions,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_types,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    view_transformations::position_world_to_clip,
}

#ifdef OIT_ENABLED
    #import bevy_core_pipeline::oit::oit_draw
#endif

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    // TODO: Somehow the data for this comes with a persistent perf penalty,
    // even with no shader code using it.
    // Investigate.
    @location(20) @interpolate(flat) flags: u32,
}

fn color_overlay(
    base: vec4<f32>,
    vert: vec4<f32>,
) -> vec4<f32> {
    return vec4(mix(base.rgb, vert.rgb, vert.a), base.a);
}

@vertex
fn vertex(
    vertex: Vertex,
    @location(20) flags: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex.instance_index
    );

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
        out.position = position_world_to_clip(out.world_position.xyz);
    #endif

    #ifdef VERTEX_COLORS
        // apply my color customization
        out.color = color_overlay(pbr_bindings::material.base_color, vertex.color);
    #endif

    #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        out.instance_index = vertex.instance_index;
    #endif

    out.flags = flags;

    return out;
}

@fragment
fn fragment(
    vertex_output: forward_io::VertexOutput,
    @location(20) @interpolate(flat) flags: u32,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var in = vertex_output;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    #ifdef VERTEX_COLORS
        let base_color = in.color;
    #else
        let base_color = pbr_input.material.base_color;
    #endif

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, base_color);

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

    let contrast = (flags & 1u) != 0;
    if contrast {
        out.color = vec4(vec3(0.0), 1.0);
    }

    return out;
}
