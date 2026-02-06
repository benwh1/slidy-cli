use std::str::FromStr as _;

use clap::{ArgGroup, Subcommand};
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{puzzle::Puzzle, size::Size},
};

use crate::enums::{ColoringType, LabelType, Metric, StateFormatter};

#[derive(Subcommand, Debug)]
pub enum Command {
    #[clap(about = "Applies algorithms to puzzle states")]
    #[clap(group(ArgGroup::new("group").multiple(true).required(true)))]
    Apply {
        #[clap(short, long, group = "group")]
        state: Option<Puzzle>,

        #[clap(short, long, group = "group")]
        alg: Option<Algorithm>,
    },

    #[clap(about = "Applies algorithms to the solved state")]
    ApplyToSolved {
        #[clap(short, long)]
        alg: Option<Algorithm>,

        #[clap(short, long)]
        size: Size,
    },

    #[clap(about = "Appends a prefix or suffix to an algorithm")]
    Concat {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        prefix: Algorithm,

        #[clap(short, long)]
        suffix: Algorithm,
    },

    #[clap(about = "Embeds a puzzle state into a larger puzzle")]
    #[clap(group(ArgGroup::new("group").multiple(true).required(true)))]
    #[clap(group(ArgGroup::new("target_type").multiple(false).required(false)))]
    Embed {
        #[clap(group = "group")]
        state: Option<Puzzle>,

        #[clap(short, long, group = "group", group = "target_type")]
        target: Option<Puzzle>,

        #[clap(short, long, group = "group", group = "target_type")]
        size: Option<Size>,
    },

    #[clap(about = "Filters out suboptimal solutions from a list of algorithms")]
    FilterOptimal {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        size: Size,

        #[clap(short, long, default_value = "stm")]
        metric: Metric,

        #[clap(short, long)]
        keep_suboptimal: bool,
    },

    #[clap(about = "Formats algorithms using long or short notation, with or without spaces")]
    Format {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        long: bool,

        #[clap(short, long)]
        spaced: bool,
    },

    #[clap(about = "Formats puzzle states inline or in a grid layout")]
    #[clap(group(ArgGroup::new("formatter")))]
    FormatState {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "inline")]
        format: StateFormatter,
    },

    #[clap(about = "Prints the scramble state, given a solution and the size of the puzzle")]
    FromSolution {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        size: Size,
    },

    #[clap(about = "Generates random scrambles")]
    #[clap(group(ArgGroup::new("scrambler")))]
    #[clap(group(ArgGroup::new("scrambler_random_moves").requires("random_moves").multiple(true)))]
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap(), value_parser = Size::from_str)]
        size: Size,

        #[clap(long, group = "scrambler", default_value_t = true)]
        random_state: bool,

        #[clap(long, group = "scrambler")]
        random_moves: bool,

        #[clap(short = 'm', long, default_value_t = 80, requires = "random_moves")]
        num_moves: u64,

        #[clap(short = 'b', long, requires = "random_moves")]
        allow_backtracking: bool,

        #[clap(short = 'i', long, requires = "random_moves")]
        allow_illegal_moves: bool,
    },

    #[clap(about = "Prints the inverse of an algorithm")]
    Invert { alg: Option<Algorithm> },

    #[clap(about = "Prints the length of an algorithm in single tile moves")]
    Length {
        alg: Option<Algorithm>,

        #[clap(short, long, default_value = "stm")]
        metric: Metric,
    },

    #[clap(
        about = "Prints the sum of the Manhattan distances of all pieces from their solved \
        positions"
    )]
    Md { state: Option<Puzzle> },

    #[clap(
        about = "Finds the difference in length between an algorithm and the optimal solution \
        of the scramble"
    )]
    OptDiff {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        size: Size,

        #[clap(short, long, default_value = "stm")]
        metric: Metric,
    },

    #[clap(
        about = "Attempts to find a shorter equivalent algorithm by optimally solving all \
        sub-algorithms of the given length"
    )]
    Optimize {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        length: u64,

        #[clap(short, long, default_value = "stm")]
        metric: Metric,
    },

    #[clap(about = "Creates an SVG image of a puzzle state")]
    Render {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "fringe")]
        label: LabelType,

        #[clap(short, long, default_value = "rainbow")]
        coloring: ColoringType,

        #[clap(short, long, default_value = "75.0")]
        tile_size: f32,

        #[clap(short = 'g', long, default_value = "0.0")]
        tile_gap: f32,

        #[clap(long, default_value = "trivial")]
        border_label: LabelType,

        #[clap(long, default_value = "black")]
        border_coloring: ColoringType,

        #[clap(short, long, default_value = "0.0")]
        border_thickness: f32,

        #[clap(short = 's', long, default_value = "30.0")]
        font_size: f32,

        #[clap(short, long)]
        output: String,
    },

    #[clap(about = "Simplifies algorithms by combining consecutive moves when possible")]
    Simplify {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(about = "Prints a sub-algorithm between two moves")]
    Slice {
        alg: Option<Algorithm>,

        #[clap(short, long, default_value = "0")]
        start: u64,

        #[clap(short, long)]
        end: Option<u64>,
    },

    #[clap(about = "Checks if puzzle states are solvable")]
    Solvable { state: Option<Puzzle> },

    #[clap(about = "Finds one optimal solution to a puzzle state")]
    Solve {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "stm")]
        metric: Metric,

        #[clap(short, long, default_value = "row-grids")]
        label: LabelType,

        #[clap(short, long)]
        verbose: bool,
    },
}
