struct SimParams {
  width: u32,
  height: u32,
  cell_dimension: u32,
  cell_number_x: u32,
  cell_number_y: u32,
  total_cell_number: u32,
  number_colors: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color_index: u32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
/// color index to the real srgb color
@group(0) @binding(1) var<storage, read> colormap: array<f32>; // size of the number of color


@vertex
fn main_vs(
    @builtin(instance_index) cell_index: u32,
    @location(0) color_index: u32,
    @location(1) vspos: vec2<f32>,
) -> VertexOutput {
  let rawcolpos = from_index_to_pos(cell_index);

  let pos = vec2<f32>(-1.0+(2.0/(f32(params.cell_number_x)-1.0))*f32(rawcolpos.x), -1.0+(2.0/(f32(params.cell_number_y)-1.0))*f32(rawcolpos.y));

  var out: VertexOutput;
  out.color_index = color_index;
  out.position = vec4<f32>(pos + vspos, 0.0, 1.0);

  return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
  let index_start = in.color_index * params.number_colors;
  return vec4<f32>(colormap[index_start], colormap[index_start+1], colormap[index_start+2], 1.0);
}

// HELPERS

fn from_index_to_pos(index: u32) -> vec2<u32> {
  return vec2<u32>(index%params.cell_number_x, index/params.cell_number_y);
}