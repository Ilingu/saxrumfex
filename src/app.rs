use nanorand::{Rng, WyRand};

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

    pub color_number: u32,
    pub colormap: Vec<f32>,
}

impl AppState {
    pub fn new(
        AppArgs {
            window_size: w,
            cell_number: wanted_cell_number,
            color_number,
        }: AppArgs,
    ) -> Self {
        let cell_area = w * w / wanted_cell_number;
        let cell_dimension = (cell_area as f64).sqrt().ceil() as u32;
        let (cwidth, cheihgt) = (w / cell_dimension, w / cell_dimension);

        let real_cell_number = cwidth * cheihgt;
        assert!(real_cell_number <= wanted_cell_number);

        let mut rng = WyRand::new();
        Self {
            width: w,
            height: w,

            cell_dimension,
            cell_number_x: cwidth,
            cell_number_y: cheihgt,

            total_cell_number: real_cell_number,
            color_number,
            colormap: (0..color_number * 3)
                .map(|_| rng.generate::<f32>())
                .collect(),
        }
    }
}

const HELP: &str = "\
SAXRUMFEX

USAGE:
  saxrumfex --window_size NUMBER --cell_number NUMBER --color_number NUMBER

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  --window_size  NUMBER - Sets window's width and height [default: 900px]
  --cell_number  NUMBER - Sets the number of cells in the simulation [default: 1000]
  --color_number NUMBER - Sets the number of distincs colors that a cell can takes [default: 3; random colors]
";

#[derive(Debug)]
pub struct AppArgs {
    pub window_size: u32,
    cell_number: u32,
    color_number: u32,
}

impl AppArgs {
    pub fn parse() -> Result<Self, pico_args::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        // Help has a higher priority and should be handled separately.
        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            std::process::exit(0);
        }

        let args = Self {
            window_size: pargs
                .opt_value_from_fn("--window_size", |s| {
                    s.parse::<u32>()
                        .map_err(|_| "'Window size' should be a valid number")
                })?
                .unwrap_or(900),
            cell_number: pargs
                .opt_value_from_fn("--cell_number", |s| {
                    s.parse::<u32>()
                        .map_err(|_| "'Cell number' should be a valid number")
                })?
                .unwrap_or(1000),
            color_number: pargs
                .opt_value_from_fn("--color_number", |s| {
                    s.parse::<u32>()
                        .map_err(|_| "'Color number' should be a valid number")
                })?
                .unwrap_or(3),
        };

        Ok(args)
    }
}
