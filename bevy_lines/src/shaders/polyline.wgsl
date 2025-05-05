#import bevy_render::view::View

@group(0) @binding(0)
var<uniform> view: View;

struct Polyline {
    model: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> polyline: Polyline;

struct PolylineMaterial {
    color: vec4<f32>,
    depth_bias: f32,
    width: f32,
};

@group(2) @binding(0)
var<uniform> material: PolylineMaterial;

struct Vertex {
    @location(0) point_a: vec3<f32>,
    @location(1) point_b: vec3<f32>,
    #ifdef POLYLINE_CONDITIONAL
        @location(2) control_point_a: vec3<f32>,
        @location(3) control_point_b: vec3<f32>,
    #endif
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Line {
    slope: f32,
    intercept: f32,
}

struct Segment2 {
    a: vec2<f32>,
    b: vec2<f32>,
}

struct Segment3 {
    a: vec3<f32>,
    b: vec3<f32>,
}

struct Segment4 {
    a: vec4<f32>,
    b: vec4<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var positions = array<vec3<f32>, 6u>(
        vec3(0.0, -0.5, 0.0),
        vec3(0.0, -0.5, 1.0),
        vec3(0.0, 0.5, 1.0),
        vec3(0.0, -0.5, 0.0),
        vec3(0.0, 0.5, 1.0),
        vec3(0.0, 0.5, 0.0)
    );
    let position = positions[vertex.index];

    let drawn_world = world_from_model(vertex.point_a, vertex.point_b);

    var drawn_clip = clip_from_world(drawn_world);
    // If one of the endpoints is behind the near plane (and thus possibly behind the camera),
    //     truncate the line segment so that it's entirely in front of the plane.
    // This avoids incorrect behavior when performing the perspective divide.
    drawn_clip = clip_segment_to_near_plane(drawn_clip);

    var drawn_screen = screen_from_clip(drawn_clip);

    #ifdef POLYLINE_CONDITIONAL
        var control_world = world_from_model(vertex.control_point_a, vertex.control_point_b);
        // Unlike the drawn segment, the control segment cannot be clipped safely,
        //     as its endpoints determine whether the main line gets drawn.
        // However, shifting it along the axis defined by the drawn segment
        //     will preserve its position relative to that segment's infinite line,
        //     so we can do that instead to guarantee everything's in front of the camera.
        control_world = shift_segment_to_near_plane(control_world, drawn_world);

        let control_clip = clip_from_world(control_world);
        let control_screen = screen_from_clip(control_clip);
        let intersects = check_opt_line_intersection(drawn_screen, control_screen);
        if intersects {
            // Prevent the line from being drawn by throwing a NaN-shaped wrench into the math.
            let zero = 0.0;
            drawn_screen.a = vec2(zero / zero);
        }
    #endif

    let x_basis = normalize(drawn_screen.b - drawn_screen.a);
    let y_basis = vec2(-x_basis.y, x_basis.x);

    var line_width = material.width;
    let clip = mix(drawn_clip.a, drawn_clip.b, position.z);
    var color = material.color;

    #ifdef POLYLINE_PERSPECTIVE
        line_width /= clip.w;
        // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
        if (line_width > 0.0 && line_width < 1.0) {
            color.a *= line_width;
            line_width = 1.0;
        }
    #endif

    let pt_offset = line_width * (position.x * x_basis + position.y * y_basis);
    let pt0 = drawn_screen.a + pt_offset;
    let pt1 = drawn_screen.b + pt_offset;
    let pt = mix(pt0, pt1, position.z);

    var depth: f32 = clip.z;
    if material.depth_bias >= 0.0 {
        depth = depth * (1.0 - material.depth_bias);
    } else {
        let epsilon = 4.88e-04;
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0
        // clip.w represents the near plane in homogenous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convinient for them
        depth = depth * exp2(-material.depth_bias * log2(clip.w / depth - epsilon));
    }

    let resolution = view.viewport.zw;
    return VertexOutput(vec4(clip.w * ((2.0 * pt) / resolution - 1.0), depth, clip.w), color);
}

fn world_from_model(world_a: vec3<f32>, world_b: vec3<f32>) -> Segment3 {
    let a = polyline.model * vec4(world_a, 1.0);
    let b = polyline.model * vec4(world_b, 1.0);
    return Segment3(a.xyz / a.w, b.xyz / b.w);
}

fn clip_from_world(seg: Segment3) -> Segment4 {
    let a = view.clip_from_world * vec4(seg.a, 1.0);
    let b = view.clip_from_world * vec4(seg.b, 1.0);
    return Segment4(a, b);
}

fn screen_from_clip(seg: Segment4) -> Segment2 {
    let res = view.viewport.zw;
    let a = res * (0.5 * seg.a.xy / seg.a.w + 0.5);
    let b = res * (0.5 * seg.b.xy / seg.b.w + 0.5);
    return Segment2(a, b);
}

fn in_bounds(a: f32, b: f32, v: f32) -> bool {
    return min(a, b) <= v && v <= max(a, b);
}

fn find_line(seg: Segment2) -> Line {
    let slope = (seg.b.y - seg.a.y) / (seg.b.x - seg.a.x);
    let intercept = seg.a.y - slope * seg.a.x;
    return Line(slope, intercept);
}

fn sample_line(f: Line, x: f32) -> vec2<f32> {
    let y = f.slope * x + f.intercept;
    return vec2(x, y);
}

fn check_opt_line_intersection(drawn: Segment2, control: Segment2) -> bool {
    let e = 0.01;
    let drawn_is_vertical = distance(drawn.a.x, drawn.b.x) < e;
    let control_is_vertical = distance(control.a.x, control.b.x) < e;
    if drawn_is_vertical && control_is_vertical {
        return false;
    } else if drawn_is_vertical {
        return in_bounds(control.a.x, control.b.x, drawn.a.x);
    } else if control_is_vertical {
        let x = control.a.x;
        let t = (x - drawn.a.x) / (drawn.b.x - drawn.a.x);
        let y = mix(drawn.a.y, drawn.b.y, t);
        return in_bounds(control.a.y, control.b.y, y);
    }

    let drawn_line = find_line(drawn);
    let control_line = find_line(control);
    let intersection = find_line_intersection(drawn_line, control_line);

    return in_bounds(control.a.x, control.b.x, intersection.x);
}

fn find_line_intersection(y1: Line, y2: Line) -> vec2<f32> {
    // y1 = m1x + b1
    // y2 = m2x + b2
    // m1x + b1 = m2x + b2
    // m1x - m2x = b2 - b1
    // (m1 - m2)x = b2 - b1
    // x = (b2 - b1) / (m1 - m2)
    let x = (y2.intercept - y1.intercept) / (y1.slope - y2.slope);
    return sample_line(y1, x);
}

fn shift_segment_to_near_plane(seg: Segment3, axis: Segment3) -> Segment3 {
    let near = view.frustum[4];
    let dist_a = dot(seg.a, near.xyz) + near.w;
    let dist_b = dot(seg.b, near.xyz) + near.w;
    // if one of the above values is positive,
    //     then that point is in front of the near plane
    // if it is negative, it is behind the plane,
    //     and the segment will need to be shifted "forward" by that amount
    // both points should be shifted based on whichever one needs a bigger nudge
    let distance = max(0.0, max(-dist_a, -dist_b));

    let direction = normalize(axis.b - axis.a);

    // this dot product is the projection of `axis` onto the near plane's normal
    // we want to move along `axis` enough to get `distance` units of projected movement along the normal
    let nudge = distance * direction / dot(direction, near.xyz);
    return Segment3(seg.a + nudge, seg.b + nudge);
}

fn clip_segment_to_near_plane(seg: Segment4) -> Segment4 {
    let a = clip_point_to_near_plane(seg.a, seg.b);
    let b = clip_point_to_near_plane(seg.b, seg.a);
    return Segment4(a, b);
}

fn clip_point_to_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    // Move a if a is behind the near plane and b is in front. 
    if a.z > a.w && b.z <= b.w {
        // Interpolate a towards b until it's at the near plane.
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        let t = distance_a / (distance_a - distance_b);
        return a + (b - a) * t;
    }
    return a;
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return in.color;
}
