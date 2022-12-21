use std::{error::Error, str::FromStr};

use clap::{ArgGroup, Subcommand, ValueEnum};
use palette::rgb::Rgba;
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        color_scheme::{Scheme, SchemeList},
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

use crate::{size::Size, util::try_func};

#[derive(Subcommand, Debug)]
pub enum Command {
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
    },
    Solvable {
        state: Option<Puzzle>,
    },
    Apply {
        state: Option<Puzzle>,

        #[clap(short, long)]
        alg: Algorithm,
    },
    LowerBound {
        state: Option<Puzzle>,
    },
    Render {
        state: Puzzle,

        #[clap(short, long, default_value = "fringe")]
        label: LabelType,

        #[clap(short, long, default_value = "rainbow-bright-full")]
        coloring: ColoringType,

        #[clap(short, long)]
        output: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LabelType {
    Fringe,
    Rows,
    Grids,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ColoringType {
    None,
    Rainbow,
    RainbowFull,
    RainbowBright,
    RainbowBrightFull,
}

pub fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
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
        Command::Solve { state } => try_func(solve, state),
        Command::Solvable { state } => try_func(solvable, state),
        Command::Apply { state, alg } => try_func(|s| apply(s, &alg), state),
        Command::LowerBound { state } => try_func(lower_bound, state),
        Command::Render {
            state,
            label,
            coloring,
            output,
        } => render(&state, label, coloring, &output),
    }
}

pub fn generate(
    number: u64,
    Size(width, height): Size,
    s: impl Scrambler,
) -> Result<(), Box<dyn Error>> {
    let mut p = Puzzle::new(width as usize, height as usize)?;

    for _ in 0..number {
        p.reset();
        s.scramble(&mut p);
        println!("{p}");
    }

    Ok(())
}

pub fn solve(state: &mut Puzzle) -> Result<(), Box<dyn Error>> {
    let mut s = Solver::new(state, &ManhattanDistance);
    let a = s.solve()?;
    println!("{a}");

    Ok(())
}

pub fn solvable(state: &mut Puzzle) {
    println!("{}", state.is_solvable());
}

pub fn apply(state: &mut Puzzle, alg: &Algorithm) {
    if state.try_apply_alg(alg) {
        println!("{state}");
    } else {
        println!("Invalid");
    }
}

pub fn lower_bound(state: &mut Puzzle) {
    if state.is_solvable() {
        let b: u64 = ManhattanDistance.bound(state);
        println!("{b}");
    } else {
        println!("Unsolvable");
    }
}

fn render(
    state: &Puzzle,
    label: LabelType,
    coloring: ColoringType,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let label: Box<dyn Label> = match label {
        LabelType::Fringe => Box::new(SplitSquareFringe),
        LabelType::Rows => Box::new(Rows),
        LabelType::Grids => Box::new(Scaled::new(RowGrids, (5, 5))?),
    };

    let coloring: Box<dyn Coloring> = match coloring {
        ColoringType::None => Box::new(Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 0.0))),
        ColoringType::Rainbow => Box::new(Rainbow),
        ColoringType::RainbowFull => Box::new(RainbowFull),
        ColoringType::RainbowBright => Box::new(RainbowBright),
        ColoringType::RainbowBrightFull => Box::new(RainbowBrightFull),
    };

    let schemes = [Scheme::new(label, coloring)];
    let scheme_list = SchemeList::new(&schemes)?;

    let renderer: Renderer<_, _> = Renderer::with_scheme(&scheme_list).text(Text::with_scheme(
        Scheme::new(Trivial, Monochrome::new(Rgba::new(0.0, 0.0, 0.0, 1.0))),
    ));

    let svg = renderer.svg(state)?;
    svg::save(output, &svg)?;

    Ok(())
}
