#[derive(Debug)]
pub struct AppState {
    /// screen width (in px)
    pub width: u32,
    /// screen height (in px)
    pub height: u32,

    /// length of the square (in px)
    pub cell_dimension: u32,
    /// the cells, containing their colors
    // pub cells: Vec<u32>,

    /// number of cells in the x direction
    pub cell_number_x: u32,
    /// number of cells in the y direction
    pub cell_number_y: u32,

    /// Real number of cell displayed (not the same as the number wanted by the user)
    pub total_cell_number: u32,
}

impl AppState {
    pub fn new(w: u32, h: u32, wanted_cell_number: u32) -> Self {
        let cell_area = w * h / wanted_cell_number;
        let cell_dimension = (cell_area as f64).sqrt().floor() as u32;
        let (cwidth, cheihgt) = (w / cell_dimension, h / cell_dimension);

        let real_cell_number = cwidth * cheihgt;
        assert!(real_cell_number <= wanted_cell_number);

        Self {
            width: w,
            height: h,

            cell_dimension,
            // cells: Vec::with_capacity(real_cell_number as usize),
            cell_number_x: cwidth,
            cell_number_y: cheihgt,

            total_cell_number: real_cell_number,
        }
    }
}
