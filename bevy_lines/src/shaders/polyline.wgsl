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
    @location(2) control_point_a: vec3<f32>,
    @location(3) control_point_b: vec3<f32>,
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

fn project_1(point: vec3<f32>) -> vec4<f32> {
    return view.clip_from_world * polyline.model * vec4(point, 1.0);
}

fn project_2(point: vec4<f32>) -> vec2<f32> {
    let resolution = view.viewport.zw;
    return resolution * (0.5 * point.xy / point.w + 0.5);
}

fn project(point: vec3<f32>) -> vec2<f32> {
    return project_2(project_1(point));
}

fn find_line(a: vec2<f32>, b: vec2<f32>) -> Line {
    let slope = (b.y - a.y) / (b.x - a.x);
    let intercept = a.y - slope * a.x;
    return Line(slope, intercept);
}

fn find_intersection(y1: Line, y2: Line) -> vec2<f32> {
    // y1 = m1x + b1
    // y2 = m2x + b2
    // m1x + b1 = m2x + b2
    // m1x - m2x = b2 - b1
    // (m1 - m2)x = b2 - b1
    // x = (b2 - b1) / (m1 - m2)
    let x = (y2.intercept - y1.intercept) / (y1.slope - y2.slope);
    let y = y1.slope * x + y1.intercept;
    return vec2(x, y);
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

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    var clip0 = project_1(vertex.point_a);
    var clip1 = project_1(vertex.point_b);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip0 = clip_near_plane(clip0, clip1);
    clip1 = clip_near_plane(clip1, clip0);

    let clip = mix(clip0, clip1, position.z);

    var screen0 = project_2(clip0);
    var screen1 = project_2(clip1);

    let x_basis = normalize(screen1 - screen0);
    let y_basis = vec2(-x_basis.y, x_basis.x);

    var line_width = material.width;
    var color = material.color;

    #ifdef POLYLINE_PERSPECTIVE
        line_width /= clip.w;
        // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
        if (line_width > 0.0 && line_width < 1.0) {
            color.a *= line_width;
            line_width = 1.0;
        }
    #endif

    if any(vertex.control_point_a != vertex.point_a) {
        let pa = project(vertex.point_a);
        let pb = project(vertex.point_b);
        let cpa = project(vertex.control_point_a);
        let cpb = project(vertex.control_point_b);
        let intersection = find_intersection(find_line(pa, pb), find_line(cpa, cpb));
        
        let x0 = min(cpa.x, cpb.x);
        let x1 = max(cpa.x, cpb.x);
        let intersects = intersection.x >= x0 && intersection.x <= x1;

        if intersects {
            // set something to NaN that will propagate to the output coord
            let a = 0.0;
            let b = 0.0;
            screen0 = vec2(a / b);
        }
    }

    let pt_offset = line_width * (position.x * x_basis + position.y * y_basis);
    let pt0 = screen0 + pt_offset;
    let pt1 = screen1 + pt_offset;
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

fn clip_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
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
