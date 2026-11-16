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
        /// Puzzle state to apply the algorithm to. If omitted, states are read from stdin, one per
        /// line.
        #[clap(short, long, group = "group")]
        state: Option<Puzzle>,

        /// Algorithm to apply. If omitted, algorithms are read from stdin, one per line.
        #[clap(short, long, group = "group")]
        alg: Option<Algorithm>,
    },

    #[clap(about = "Applies algorithms to the solved state")]
    ApplyToSolved {
        /// Algorithm to apply. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Size of the puzzle.
        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,
    },

    #[clap(about = "Generates shell completion scripts")]
    Completions {
        /// Shell to generate the completion script for.
        shell: Shell,
    },

    #[clap(about = "Prepends a prefix and appends a suffix to an algorithm")]
    Concat {
        /// Algorithm to extend. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Algorithm to put before each algorithm.
        #[clap(short, long)]
        prefix: Algorithm,

        /// Algorithm to put after each algorithm.
        #[clap(short, long)]
        suffix: Algorithm,
    },

    #[clap(about = "Embeds a puzzle state into a larger puzzle")]
    #[clap(group(ArgGroup::new("group").multiple(true).required(true)))]
    #[clap(group(ArgGroup::new("target_type").multiple(false).required(false)))]
    Embed {
        /// Puzzle state to embed. If omitted, states are read from stdin, one per line.
        #[clap(group = "group")]
        state: Option<Puzzle>,

        /// Puzzle state to embed into. If neither this nor --size is given, targets are read from
        /// stdin, one per line.
        #[clap(short, long, group = "group", group = "target_type")]
        target: Option<Puzzle>,

        /// Embed into the solved state of this size.
        #[clap(short, long, group = "group", group = "target_type")]
        size: Option<Size>,
    },

    #[clap(about = "Filters out suboptimal solutions from a list of algorithms")]
    FilterOptimal {
        /// Solution to check. If omitted, solutions are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Size of the puzzle the solutions are for.
        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        /// Metric to measure solution lengths in.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,

        /// Keep only the suboptimal solutions instead.
        #[clap(short, long)]
        keep_suboptimal: bool,
    },

    #[clap(about = "Filters out unsolvable puzzle states")]
    FilterSolvable {
        /// Puzzle state to check. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,

        /// Keep only the unsolvable states instead.
        #[clap(short, long)]
        unsolvable: bool,
    },

    #[clap(about = "Formats algorithms using long or short notation, with or without spaces")]
    Format {
        /// Algorithm to format. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Write repeated moves out in full (`RRR`) instead of with a count (`R3`).
        #[clap(short, long)]
        long: bool,

        /// Separate moves with spaces.
        #[clap(short, long)]
        spaced: bool,
    },

    #[clap(about = "Formats puzzle states inline or in a grid layout")]
    FormatState {
        /// Puzzle state to format. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,

        /// Output layout.
        #[clap(short, long, value_enum, default_value_t = StateFormatter::Inline)]
        format: StateFormatter,
    },

    #[clap(about = "Prints the scramble state, given a solution and the size of the puzzle")]
    FromSolution {
        /// Solution of the scramble. If omitted, solutions are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Size of the puzzle.
        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,
    },

    #[clap(about = "Generates random scrambles")]
    Generate {
        /// Number of scrambles to generate.
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        /// Size of the puzzle.
        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        /// Seed for the random number generator, for reproducible scrambles.
        #[clap(long)]
        seed: Option<u64>,

        /// Scramble with random moves instead of picking a uniformly random solvable state.
        #[clap(long)]
        random_moves: bool,

        /// Number of random moves to make (with --random-moves).
        #[clap(short = 'm', long, default_value_t = 80, requires = "random_moves")]
        num_moves: u64,

        /// Allow a move to undo the previous one (with --random-moves).
        #[clap(short = 'b', long, requires = "random_moves")]
        allow_backtracking: bool,

        /// Count attempted moves that can't be made, which then do nothing (with --random-moves).
        #[clap(short = 'i', long, requires = "random_moves")]
        allow_illegal_moves: bool,
    },

    #[clap(about = "Prints the inverse of an algorithm")]
    Invert {
        /// Algorithm to invert. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,
    },

    #[clap(about = "Prints the length of an algorithm")]
    Length {
        /// Algorithm to measure. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Metric to measure the length in.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(
        about = "Prints the sum of the Manhattan distances of all pieces from their solved \
        positions"
    )]
    Md {
        /// Puzzle state to measure. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,
    },

    #[clap(
        about = "Finds the difference in length between an algorithm and the optimal solution \
        of the scramble"
    )]
    OptDiff {
        /// Solution to compare. If omitted, solutions are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Size of the puzzle the solutions are for.
        #[clap(short, long, default_value_t = Size::new(4, 4).unwrap())]
        size: Size,

        /// Metric to measure lengths in.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(
        about = "Attempts to find a shorter equivalent algorithm by optimally solving all \
        sub-algorithms of the given length"
    )]
    Optimize {
        /// Algorithm to optimize. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Length of the sub-algorithms to optimize, in the chosen metric. Larger values find
        /// more improvements but are slower.
        #[clap(short, long)]
        length: u64,

        /// Metric to measure lengths in.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(about = "Prints the piece in a given position in a puzzle state")]
    PieceAt {
        /// Puzzle state to look in. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,

        /// Position to look at, counting from 0 in reading order (left to right, then top to
        /// bottom).
        #[clap(short, long)]
        position: u64,
    },

    #[clap(about = "Prints the position of a piece in a puzzle state")]
    PiecePosition {
        /// Puzzle state to look in. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,

        /// Piece to find. The gap is piece 0.
        ///
        /// The position is printed counting from 0 in reading order (left to right, then top to
        /// bottom).
        #[clap(short, long)]
        piece: u64,
    },

    #[clap(about = "Creates an SVG image of a puzzle state")]
    Render {
        /// Puzzle state to render. If omitted, a single state is read from stdin.
        state: Option<Puzzle>,

        /// Label used to colour the tiles: tiles with the same label get the same colour.
        #[clap(short, long, value_enum, default_value_t = LabelType::Fringe)]
        label: LabelType,

        /// Colouring applied to the tile labels.
        #[clap(short, long, value_enum, default_value_t = ColoringType::Rainbow)]
        coloring: ColoringType,

        /// Width and height of each tile, in pixels.
        #[clap(short, long, default_value_t = 75.0)]
        tile_size: f32,

        /// Space between tiles, in pixels.
        #[clap(short = 'g', long, default_value_t = 0.0)]
        tile_gap: f32,

        /// Label used to colour the tile borders.
        #[clap(long, value_enum, default_value_t = LabelType::Trivial)]
        border_label: LabelType,

        /// Colouring applied to the border labels.
        #[clap(long, value_enum, default_value_t = ColoringType::Black)]
        border_coloring: ColoringType,

        /// Thickness of the tile borders, in pixels.
        #[clap(short, long, default_value_t = 0.0)]
        border_thickness: f32,

        /// Font size of the tile numbers, in pixels.
        #[clap(short = 's', long, default_value_t = 30.0)]
        font_size: f32,

        /// File to write the SVG image to.
        #[clap(short, long)]
        output: String,
    },

    #[clap(about = "Simplifies algorithms by combining consecutive moves when possible")]
    Simplify {
        /// Algorithm to simplify. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Also print the lengths before and after simplifying.
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(about = "Prints a sub-algorithm between two moves")]
    Slice {
        /// Algorithm to slice. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,

        /// Number of moves to skip from the start of the algorithm.
        #[clap(short, long, default_value_t = 0)]
        start: u64,

        /// Number of moves from the start of the algorithm to stop at (exclusive). Defaults to
        /// the end of the algorithm.
        #[clap(short, long)]
        end: Option<u64>,

        /// Metric used to count moves for --start and --end.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,
    },

    #[clap(about = "Checks if puzzle states are solvable")]
    Solvable {
        /// Puzzle state to check. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,
    },

    #[clap(about = "Finds solutions to a puzzle state")]
    Solve {
        /// Puzzle state to solve. If omitted, states are read from stdin, one per line.
        state: Option<Puzzle>,

        /// Metric to find optimal solutions in.
        #[clap(short, long, value_enum, default_value_t = Metric::Stm)]
        metric: Metric,

        /// What counts as solved: every tile must be in a position with the same label as its
        /// solved position. The default, row-grids, solves the whole puzzle.
        #[clap(short, long, value_enum, default_value_t = LabelType::RowGrids)]
        label: LabelType,

        /// Search depth to start from.
        #[clap(long, default_value_t = 0)]
        min_depth: u64,

        /// Search depth to stop at (inclusive). Defaults to unlimited.
        #[clap(long)]
        max_depth: Option<u64>,

        /// Number of solutions to find.
        #[clap(
            short = 'n',
            long,
            default_value_t = 1,
            default_value_if("all_optimal", "true", "18446744073709551615")
        )]
        num_solutions: u64,

        /// Stop searching this many moves past the length of the first solution found. Defaults
        /// to unlimited.
        #[clap(long, default_value_if("all_optimal", "true", "0"))]
        depth_beyond_optimal: Option<u64>,

        /// Find every optimal solution.
        #[clap(long, conflicts_with_all = ["num_solutions", "depth_beyond_optimal"])]
        all_optimal: bool,

        /// Print a line to stderr after each search depth is finished.
        #[clap(long)]
        show_progress: bool,

        /// Also print the length of each solution.
        #[clap(short, long)]
        verbose: bool,
    },

    #[clap(about = "Prints the solved state")]
    SolvedState {
        /// Size of the puzzle. If omitted, sizes are read from stdin, one per line.
        size: Option<Size>,
    },

    #[clap(about = "Prints the transpose of an algorithm")]
    Transpose {
        /// Algorithm to transpose. If omitted, algorithms are read from stdin, one per line.
        alg: Option<Algorithm>,
    },
}
