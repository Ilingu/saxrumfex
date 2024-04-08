struct SimParams {
  width: u32,
  height: u32,
  cell_dimension: u32,
  cell_number_x: u32,
  cell_number_y: u32,
  total_cell_number: u32,
  number_colors: u32,
};

/// Cells are an unidimentional array (array<Cell>) to simplify data structure
/// that why, we also transfer width, height and total number of cells which must be constant through the simulation

@group(0) @binding(0) var<uniform> params: SimParams;
/// frame input
@group(0) @binding(1) var<storage, read> cellSrc: array<u32>;
/// frame output
@group(0) @binding(2) var<storage, read_write> cellDst: array<u32>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let index = global_invocation_id.x; // cell index
  if (index >= params.total_cell_number) {
    return;
  }

  // read 
  var cell_color: u32 = cellSrc[index];
  let pos = from_index_to_pos(index);

  let x = i32(pos.x);
  let y = i32(pos.y);

  var best_enemy_color = cell_color;
  var number_of_best_enemy: u32 = 0;

  var enemies: u32 = number_array_with_capacity(params.number_colors);
  // Initialize enemy count to 0
  for (var i = 0u; i < params.number_colors; i++) {
      enemies = number_array_set(enemies, i, 0u);
  }

  for (var xi = 0u; xi < 3; xi++) {
    for (var yi = 0u; yi < 3; xi++) {
      let xoff: i32 = i32(xi)-1;
      let yoff: i32 = i32(yi)-1;

      // skip current cell
      if xoff == 0 && yoff == 0 {
        continue;
      }

      // neighbor position
      let xnei = x+xoff;
      let ynei = y+yoff;

      // check if out of bound
      if xnei < 0 || xnei >= i32(params.cell_number_x){
        continue;
      }
      if ynei < 0 || ynei >= i32(params.cell_number_y) {
        continue;
      }

      let neighbor_index = from_pos_to_index(u32(xnei), u32(ynei));

      // check if out of bound #2 (should be useless and removed)
      if neighbor_index < 0 || neighbor_index >= params.total_cell_number {
        continue;
      }

      let neighbor_color = cellSrc[neighbor_index];
      if is_enemy(cell_color, neighbor_color) {
        enemies = number_array_set(enemies, neighbor_color, number_array_get(enemies, neighbor_color)+1u);
        if number_array_get(enemies, neighbor_color) > number_of_best_enemy {
          best_enemy_color = neighbor_color;
          number_of_best_enemy = number_array_get(enemies, neighbor_color);
        }
      }
    }
  }

  if number_of_best_enemy >= 2 {
    cell_color = best_enemy_color;
  }

  // Write back
  cellDst[index] = cell_color;
}

// HELPERS

fn is_enemy(your_color: u32, other_color: u32) -> bool {
  return (your_color+1)%params.number_colors == other_color;
}

fn from_index_to_pos(index: u32) -> vec2<u32> {
  return vec2<u32>(index%params.cell_number_x, index/params.cell_number_y);
}

fn from_pos_to_index(col: u32, raw: u32) -> u32 {
  return raw * params.cell_number_y + col;
}

// array in number

fn number_array_with_capacity(n: u32) -> u32 {
    return power(10u, n);
}

fn power(base: u32, exponent: u32) -> u32 {
    var result: u32 = 1;
    var x: u32 = base;
    var n: u32 = exponent;

    while (n != 0) {
        if ((n & 1) != 0) {
            result *= x;
        }
        x *= x;
        n = n >> 1;
    }

    return result;
}

/// no out of bound checking
fn number_array_get(arr: u32, index: u32) -> u32 {
    return (arr / power(10u, index)) % 10;
}

fn number_array_set(arr: u32, index: u32, new_num: u32) -> u32 {
    var new_arr: u32 = arr;

    let curr_num: u32 = number_array_get(arr, index);
    let diff: i32 = i32(new_num) - i32(curr_num);
    if diff < 0 {
        new_arr -= u32(-diff) * power(10u, index);
    } else {
        new_arr += u32(diff) * power(10u, index);
    }
    return new_arr;
}
