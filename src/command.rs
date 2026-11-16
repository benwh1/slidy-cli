use clap::{ArgGroup, Subcommand};
use clap_complete::aot::Shell;
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
        alg: Option<Algorithm>,

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,
    },

    #[clap(about = "Generates shell completion scripts")]
    Completions { shell: Shell },

    #[clap(about = "Prepends a prefix and appends a suffix to an algorithm")]
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

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,

        #[clap(short, long)]
        keep_suboptimal: bool,
    },

    #[clap(about = "Filters out unsolvable puzzle states")]
    FilterSolvable {
        state: Option<Puzzle>,

        #[clap(short, long)]
        unsolvable: bool,
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
    FormatState {
        state: Option<Puzzle>,

        #[clap(short, long, value_enum, default_value_t = StateFormatter::Inline)]
        format: StateFormatter,
    },

    #[clap(about = "Prints the scramble state, given a solution and the size of the puzzle")]
    FromSolution {
        alg: Option<Algorithm>,

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,
    },

    #[clap(about = "Generates random scrambles")]
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        #[clap(long)]
        seed: Option<u64>,

        #[clap(long)]
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

    #[clap(about = "Prints the length of an algorithm")]
    Length {
        alg: Option<Algorithm>,

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
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

        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
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

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(about = "Prints the piece in a given position in a puzzle state")]
    PieceAt {
        state: Option<Puzzle>,

        #[clap(short, long)]
        position: u64,
    },

    #[clap(about = "Prints the position of a piece in a puzzle state")]
    PiecePosition {
        state: Option<Puzzle>,

        #[clap(short, long)]
        piece: u64,
    },

    #[clap(about = "Creates an SVG image of a puzzle state")]
    Render {
        state: Option<Puzzle>,

        #[clap(short, long, value_enum, default_value_t = LabelType::Fringe)]
        label: LabelType,

        #[clap(short, long, value_enum, default_value_t = ColoringType::Rainbow)]
        coloring: ColoringType,

        #[clap(short, long, default_value_t = 75.0)]
        tile_size: f32,

        #[clap(short = 'g', long, default_value_t = 0.0)]
        tile_gap: f32,

        #[clap(long, value_enum, default_value_t = LabelType::Trivial)]
        border_label: LabelType,

        #[clap(long, value_enum, default_value_t = ColoringType::Black)]
        border_coloring: ColoringType,

        #[clap(short, long, default_value_t = 0.0)]
        border_thickness: f32,

        #[clap(short = 's', long, default_value_t = 30.0)]
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

        #[clap(short, long, default_value_t = 0)]
        start: u64,

        #[clap(short, long)]
        end: Option<u64>,

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(about = "Checks if puzzle states are solvable")]
    Solvable { state: Option<Puzzle> },

    #[clap(about = "Finds solutions to a puzzle state")]
    Solve {
        state: Option<Puzzle>,

        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,

        #[clap(short, long, value_enum, default_value_t = LabelType::RowGrids)]
        label: LabelType,

        #[clap(long, default_value_t = 0)]
        min_depth: u64,

        #[clap(long, default_value_t = u64::MAX)]
        max_depth: u64,

        #[clap(
            short = 'n',
            long,
            default_value_t = 1,
            default_value_if("all_optimal", "true", "18446744073709551615")
        )]
        num_solutions: u64,

        #[clap(
            long,
            default_value_t = u64::MAX,
            default_value_if("all_optimal", "true", "0")
        )]
        depth_beyond_optimal: u64,

        #[clap(long, conflicts_with_all = ["num_solutions", "depth_beyond_optimal"])]
        all_optimal: bool,

        #[clap(long)]
        show_progress: bool,

        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(about = "Prints the solved state")]
    SolvedState { size: Option<Size> },

    #[clap(about = "Prints the transpose of an algorithm")]
    Transpose { alg: Option<Algorithm> },
}
