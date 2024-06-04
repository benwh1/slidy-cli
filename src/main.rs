#![feature(int_roundings)]

mod util;

use std::{error::Error, rc::Rc, str::FromStr};

use clap::{command, ArgGroup, Parser, Subcommand, ValueEnum};
use palette::rgb::Rgba;
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        color_scheme::{tiled::Tiled, ColorScheme, Scheme, SchemeList},
        coloring::{Coloring, Monochrome, Rainbow},
        label::{
            labels::{Fringe, Label, RowGrids, Rows, SplitFringe},
            scaled::Scaled,
        },
        puzzle::Puzzle,
        render::{Renderer, RendererBuilder, Text},
        scrambler::{RandomMoves, RandomState, Scrambler},
        size::Size,
        sliding_puzzle::SlidingPuzzle,
    },
    solver::{
        heuristic::{manhattan::ManhattanDistance, Heuristic},
        solver::Solver,
    },
};

use crate::util::{loop_func, try_func, try_func_once};

#[derive(Parser, Debug)]
#[command(
    author, version, about, long_about = None,
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true,
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(
        about = "Applies algorithms to puzzle states. If only an algorithm is given, puzzle states \
        are read from stdin. If only a puzzle state is given, algorithms are read from stdin"
    )]
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

    #[clap(
        about = "Embeds a puzzle state into a larger puzzle, e.g. a 3x3 into the bottom right \
                 corner of a 4x4"
    )]
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
        about = "Prints the lower bound on the optimal solution length using the Manhattan \
        distance heuristic"
    )]
    LowerBound { state: Option<Puzzle> },

    #[clap(
        about = "Attempts to find a shorter equivalent algorithm by optimally solving all \
        sub-algorithms of the given length"
    )]
    Optimize {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        length: u64,
    },

    #[clap(about = "Creates an SVG image of a puzzle state")]
    Render {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "fringe")]
        label: LabelType,

        #[clap(short, long, default_value = "rainbow-bright-full")]
        coloring: ColoringType,

        #[clap(short, long, default_value = "75.0")]
        tile_size: f32,

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

        #[clap(short, long, default_value = "row-grids")]
        label: LabelType,

        #[clap(short, long)]
        verbose: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum LabelType {
    RowGrids,
    Fringe,
    SplitFringe,
    Rows,
    Grids,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColoringType {
    None,
    Rainbow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum StateFormatter {
    Inline,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Metric {
    Stm,
    Mtm,
}

fn apply(state: &mut Puzzle, alg: &Algorithm) {
    if state.try_apply_alg(alg) {
        println!("{state}");
    } else {
        println!("Invalid");
    }
}

fn apply_to_solved(alg: &Algorithm, size: Size) -> Result<(), Box<dyn Error>> {
    let mut state = Puzzle::new(size);
    apply(&mut state, alg);

    Ok(())
}

fn concat(alg: &mut Algorithm, prefix: &Algorithm, suffix: &Algorithm) {
    println!("{prefix}{alg}{suffix}");
}

fn embed(state: &Puzzle, target: &mut Puzzle) {
    if state.try_embed_into(target) {
        println!("{target}");
    } else {
        println!("Invalid");
    }
}

fn format(alg: &mut Algorithm, long: bool, spaced: bool) {
    let s = match (long, spaced) {
        (true, true) => alg.display_long_spaced().to_string(),
        (true, false) => alg.display_long_unspaced().to_string(),
        (false, true) => alg.display_short_spaced().to_string(),
        (false, false) => alg.display_short_unspaced().to_string(),
    };
    println!("{s}");
}

fn format_state(state: &Puzzle, formatter: StateFormatter) {
    match formatter {
        StateFormatter::Inline => println!("{}", state.display_inline()),
        StateFormatter::Grid => println!("{}", state.display_grid()),
    }
}

fn generate(number: u64, size: Size, s: impl Scrambler) -> Result<(), Box<dyn Error>> {
    let mut p = Puzzle::new(size);

    for _ in 0..number {
        p.reset();
        s.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}

fn invert(alg: &mut Algorithm) {
    alg.invert();
    println!("{alg}");
}

fn length(alg: &mut Algorithm, metric: Metric) {
    let len: u64 = match metric {
        Metric::Stm => alg.len_stm(),
        Metric::Mtm => alg.len_mtm(),
    };
    println!("{len}");
}

fn lower_bound(state: &mut Puzzle) {
    if state.is_solvable() {
        let b: u64 = ManhattanDistance(&RowGrids).bound(state);
        println!("{b}");
    } else {
        println!("Unsolvable");
    }
}

fn optimize(alg: &mut Algorithm, length: u64) -> Result<(), Box<dyn Error>> {
    let mut idx = 0;
    while idx + length <= alg.len_stm() {
        let slice = alg.try_slice(idx..idx + length)?;
        let Some(size) = slice.min_applicable_size() else {
            idx += 1;
            continue;
        };
        let mut puzzle = Puzzle::new(size);
        puzzle.apply_alg(&slice);

        let mut solver = Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let solution = solver.solve(&puzzle)?;

        if solution.len_stm::<u64>() == length {
            idx += 1;
        } else {
            let mut start = Algorithm::from(alg.try_slice(0..idx)?);
            let middle = solution.inverse();
            let end = Algorithm::from(alg.try_slice(idx + length..alg.len_stm())?);
            start += middle;
            start += end;

            *alg = start;
        }
    }

    println!("{alg}");

    Ok(())
}

fn render(
    state: &Puzzle,
    label_type: LabelType,
    coloring_type: ColoringType,
    tile_size: f32,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let grid_size = {
        let (width, height) = state.size().into();
        (width.div_ceil(2), height.div_ceil(2))
    };

    let label: Box<dyn Label> = match label_type {
        LabelType::RowGrids => Box::new(RowGrids),
        LabelType::Fringe => Box::new(Fringe),
        LabelType::SplitFringe => Box::new(SplitFringe),
        LabelType::Rows => Box::new(Rows),
        LabelType::Grids => Box::new(Scaled::new(RowGrids, grid_size)?),
    };

    let coloring: Rc<dyn Coloring> = match coloring_type {
        ColoringType::None => Rc::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 0.0))),
        ColoringType::Rainbow => Rc::new(Rainbow::default()),
    };

    let mut schemes: Vec<Box<dyn ColorScheme>> =
        vec![Box::new(Scheme::new(label, coloring.clone()))];
    if label_type == LabelType::Grids {
        let grid_size = Size::new(grid_size.0, grid_size.1)?;

        schemes.push(Box::new(Tiled::new(
            Scheme::new(SplitFringe, coloring.clone()),
            grid_size,
        )))
    }

    let scheme_list = SchemeList::new(&schemes)?;

    let renderer: Renderer<_, _, _> = RendererBuilder::with_scheme(&scheme_list)
        .text(Text::default().font_size(tile_size * 30.0 / 75.0))
        .tile_size(tile_size)
        .build();

    let svg = renderer.render(state)?;
    svg::save(output, &svg)?;

    Ok(())
}

fn simplify(alg: &mut Algorithm, verbose: bool) {
    let orig: u64 = alg.len_stm();
    alg.simplify();
    let new: u64 = alg.len_stm();

    println!("{alg}");
    if verbose {
        println!("Original length: {orig}");

        let diff = orig - new;
        let percent = diff as f32 * 100.0 / orig as f32;
        println!("New length: {new} [-{diff}, -{percent:.4}%]",);
    }
}

fn slice(alg: &mut Algorithm, start: u64, end: Option<u64>) -> Result<(), Box<dyn Error>> {
    let end = end.unwrap_or(alg.len_stm());
    let slice = alg.try_slice(start..end)?;
    println!("{slice}");

    Ok(())
}

fn solvable(state: &mut Puzzle) {
    println!("{}", state.is_solvable());
}

fn solve(state: &mut Puzzle, label: LabelType, verbose: bool) -> Result<(), Box<dyn Error>> {
    let a = match label {
        LabelType::RowGrids => {
            let mut s = Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
            s.solve(state)?
        }
        LabelType::Rows => {
            let mut s = Solver::new(&ManhattanDistance(&Rows), &Rows);
            s.solve(state)?
        }
        LabelType::Fringe => {
            let mut s = Solver::new(&ManhattanDistance(&Fringe), &Fringe);
            s.solve(state)?
        }
        LabelType::SplitFringe => {
            let mut s = Solver::new(&ManhattanDistance(&SplitFringe), &SplitFringe);
            s.solve(state)?
        }
        LabelType::Grids => unimplemented!(),
    };

    println!("{a}");

    if verbose {
        println!("{} moves", a.len_stm::<u64>());
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Args::parse();

    match args.command {
        Command::Apply { state, alg } => match (state, alg) {
            (None, None) => unreachable!(),
            (None, Some(alg)) => loop_func(|s| apply(s, &alg)),
            (Some(state), None) => loop_func(|a| apply(&mut state.clone(), a)),
            (Some(mut state), Some(alg)) => {
                apply(&mut state, &alg);
                Ok(())
            }
        },
        Command::ApplyToSolved { alg, size } => try_func(|a| apply_to_solved(a, size), alg),
        Command::Concat {
            alg,
            prefix,
            suffix,
        } => try_func(|a| concat(a, &prefix, &suffix), alg),
        Command::Embed {
            state,
            target,
            size,
        } => {
            let target = size.map(Puzzle::new).or(target);

            match (state, target) {
                (None, None) => unreachable!(),
                (None, Some(target)) => loop_func(|s| embed(s, &mut target.clone())),
                (Some(state), None) => loop_func(|t| embed(&state.clone(), t)),
                (Some(state), Some(mut target)) => {
                    embed(&state, &mut target);
                    Ok(())
                }
            }
        }
        Command::Format { alg, long, spaced } => try_func(|a| format(a, long, spaced), alg),
        Command::FormatState { state, format } => try_func(|s| format_state(s, format), state),
        Command::Generate {
            number,
            size,
            random_moves,
            num_moves,
            allow_backtracking,
            allow_illegal_moves,
            ..
        } => {
            if random_moves {
                generate(
                    number,
                    size,
                    RandomMoves {
                        moves: num_moves,
                        allow_backtracking,
                        allow_illegal_moves,
                    },
                )
            } else {
                generate(number, size, RandomState)
            }
        }
        Command::Invert { alg } => try_func(invert, alg),
        Command::Length { alg, metric } => try_func(|a| length(a, metric), alg),
        Command::LowerBound { state } => try_func(lower_bound, state),
        Command::Optimize { alg, length } => try_func(|a| optimize(a, length), alg),
        Command::Render {
            state,
            label,
            coloring,
            tile_size,
            output,
        } => try_func_once(|s| render(s, label, coloring, tile_size, &output), state),
        Command::Simplify { alg, verbose } => try_func(|a| simplify(a, verbose), alg),
        Command::Slice { alg, start, end } => try_func(|a| slice(a, start, end), alg),
        Command::Solvable { state } => try_func(solvable, state),
        Command::Solve {
            state,
            label,
            verbose,
        } => try_func(|s| solve(s, label, verbose), state),
    }
}
