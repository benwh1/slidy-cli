#![feature(int_roundings)]

mod size;
mod util;

use std::{error::Error, rc::Rc, str::FromStr};

use clap::{command, ArgGroup, Parser, Subcommand, ValueEnum};
use palette::rgb::Rgba;
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        color_scheme::{tiled::Tiled, ColorScheme, Scheme, SchemeList},
        coloring::{Coloring, Monochrome, Rainbow, RainbowBright, RainbowBrightFull, RainbowFull},
        label::{
            labels::{Label, RowGrids, Rows, SplitSquareFringe, Trivial},
            scaled::Scaled,
        },
        puzzle::Puzzle,
        render::{Renderer, Text},
        scrambler::{RandomMoves, RandomState, Scrambler},
        sliding_puzzle::SlidingPuzzle,
    },
    solver::{
        heuristic::{Heuristic, ManhattanDistance},
        solver::Solver,
    },
};

use crate::{
    size::Size,
    util::{try_func, try_func_once},
};

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
    #[clap(group(ArgGroup::new("scrambler")))]
    #[clap(group(ArgGroup::new("scrambler_random_moves").requires("random_moves").multiple(true)))]
    Generate {
        #[clap(short, long, default_value_t = 1)]
        number: u64,

        #[clap(short, long, default_value_t = Size(4, 4), value_parser = Size::from_str)]
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
    Solve {
        state: Option<Puzzle>,

        #[clap(short, long)]
        verbose: bool,
    },
    Solvable {
        state: Option<Puzzle>,
    },
    #[clap(about = "Applies a fixed algorithm to one or multiple puzzle states")]
    Apply {
        state: Option<Puzzle>,

        #[clap(short, long)]
        alg: Algorithm,
    },
    LowerBound {
        state: Option<Puzzle>,
    },
    Render {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "fringe")]
        label: LabelType,

        #[clap(short, long, default_value = "rainbow-bright-full")]
        coloring: ColoringType,

        #[clap(short, long)]
        output: String,
    },
    Length {
        alg: Option<Algorithm>,
    },
    Simplify {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        verbose: bool,
    },
    Invert {
        alg: Option<Algorithm>,
    },
    Concat {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        prefix: Algorithm,

        #[clap(short, long)]
        suffix: Algorithm,
    },
    Format {
        alg: Option<Algorithm>,

        #[clap(short, long)]
        long: bool,

        #[clap(short, long)]
        spaced: bool,
    },
    #[clap(about = "Applies one or multiple algorithms to a fixed puzzle state")]
    #[clap(group(ArgGroup::new("puzzle").required(true)))]
    ApplyAlgs {
        alg: Option<Algorithm>,

        #[clap(short = 'p', long, group = "puzzle")]
        state: Option<Puzzle>,

        #[clap(short = 's', long, group = "puzzle")]
        size: Option<Size>,
    },
    #[clap(group(ArgGroup::new("formatter")))]
    FormatState {
        state: Option<Puzzle>,

        #[clap(short, long, default_value = "inline")]
        format: StateFormatter,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum LabelType {
    Fringe,
    Rows,
    Grids,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColoringType {
    None,
    Rainbow,
    RainbowFull,
    RainbowBright,
    RainbowBrightFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum StateFormatter {
    Inline,
    Grid,
}

fn generate(
    number: u64,
    Size(width, height): Size,
    s: impl Scrambler<Puzzle, u32>,
) -> Result<(), Box<dyn Error>> {
    let mut p = Puzzle::new(width as usize, height as usize)?;

    for _ in 0..number {
        p.reset();
        s.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}

fn solve(state: &mut Puzzle, verbose: bool) -> Result<(), Box<dyn Error>> {
    let mut s = Solver::new(state, &ManhattanDistance);
    let a = s.solve()?;
    println!("{a}");

    if verbose {
        println!("{} moves", a.len());
    }

    Ok(())
}

fn solvable(state: &mut Puzzle) {
    println!("{}", state.is_solvable());
}

fn apply(state: &mut Puzzle, alg: &Algorithm) {
    if state.try_apply_alg(alg) {
        println!("{state}");
    } else {
        println!("Invalid");
    }
}

fn lower_bound(state: &mut Puzzle) {
    if state.is_solvable() {
        let b: u64 = ManhattanDistance.bound(state);
        println!("{b}");
    } else {
        println!("Unsolvable");
    }
}

fn render(
    state: &Puzzle,
    label_type: LabelType,
    coloring_type: ColoringType,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let grid_size = (
        state.width().div_ceil(2) as u32,
        state.height().div_ceil(2) as u32,
    );

    let label: Box<dyn Label> = match label_type {
        LabelType::Fringe => Box::new(SplitSquareFringe),
        LabelType::Rows => Box::new(Rows),
        LabelType::Grids => Box::new(Scaled::new(RowGrids, grid_size)?),
    };

    let coloring: Rc<dyn Coloring> = match coloring_type {
        ColoringType::None => Rc::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 0.0))),
        ColoringType::Rainbow => Rc::new(Rainbow),
        ColoringType::RainbowFull => Rc::new(RainbowFull),
        ColoringType::RainbowBright => Rc::new(RainbowBright),
        ColoringType::RainbowBrightFull => Rc::new(RainbowBrightFull),
    };

    let mut schemes: Vec<Box<dyn ColorScheme>> =
        vec![Box::new(Scheme::new(label, coloring.clone()))];
    if label_type == LabelType::Grids {
        schemes.push(Box::new(Tiled::new(
            Scheme::new(SplitSquareFringe, coloring.clone()),
            grid_size,
        )?))
    }

    let scheme_list = SchemeList::new(&schemes)?;

    let renderer: Renderer<_, _> = Renderer::with_scheme(&scheme_list).text(Text::with_scheme(
        Scheme::new(Trivial, Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 1.0))),
    ));

    let svg = renderer.svg(state)?;
    svg::save(output, &svg)?;

    Ok(())
}

fn length(alg: &mut Algorithm) {
    println!("{}", alg.len());
}

fn simplify(alg: &mut Algorithm, verbose: bool) {
    let orig = alg.len();
    alg.simplify();
    let new = alg.len();

    println!("{alg}");
    if verbose {
        println!("Original length: {orig}");

        let diff = orig - new;
        let percent = diff as f32 * 100.0 / orig as f32;
        println!("New length: {new} [-{diff}, -{percent:.4}%]",);
    }
}

fn invert(alg: &mut Algorithm) {
    alg.invert();
    println!("{alg}");
}

fn concat(alg: &mut Algorithm, prefix: &Algorithm, suffix: &Algorithm) {
    println!("{prefix}{alg}{suffix}");
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

fn apply_algs(
    alg: &Algorithm,
    state: Option<Puzzle>,
    size: Option<Size>,
) -> Result<(), Box<dyn Error>> {
    if let Some(mut state) = state {
        apply(&mut state, alg);
    } else if let Some(size) = size {
        let mut state = Puzzle::new(size.0 as usize, size.1 as usize)?;
        apply(&mut state, alg);
    }

    Ok(())
}

fn format_state(state: &Puzzle, formatter: StateFormatter) {
    match formatter {
        StateFormatter::Inline => println!("{}", state.display_inline()),
        StateFormatter::Grid => println!("{}", state.display_grid()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command {
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
        Command::Solve { state, verbose } => try_func(|s| solve(s, verbose), state),
        Command::Solvable { state } => try_func(solvable, state),
        Command::Apply { state, alg } => try_func(|s| apply(s, &alg), state),
        Command::LowerBound { state } => try_func(lower_bound, state),
        Command::Render {
            state,
            label,
            coloring,
            output,
        } => try_func_once(|s| render(s, label, coloring, &output), state),
        Command::Length { alg } => try_func(length, alg),
        Command::Simplify { alg, verbose } => try_func(|a| simplify(a, verbose), alg),
        Command::Invert { alg } => try_func(invert, alg),
        Command::Concat {
            alg,
            prefix,
            suffix,
        } => try_func(|a| concat(a, &prefix, &suffix), alg),
        Command::Format { alg, long, spaced } => try_func(|a| format(a, long, spaced), alg),
        Command::ApplyAlgs { alg, state, size } => {
            try_func(|a| apply_algs(a, state.clone(), size), alg)
        }
        Command::FormatState { state, format } => try_func(|s| format_state(s, format), state),
    }
}
