use std::error::Error;

use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        color_scheme::{tiled::Tiled, ColorScheme, Scheme},
        label::label::{
            Checkerboard, Diagonals, Fringe, RowGrids, Rows, SplitFringe, SplitSquareFringe,
            SquareFringe, Trivial,
        },
        puzzle::Puzzle,
        render::{Borders, RendererBuilder, Text},
        scrambler::{RandomMoves, RandomState, Scrambler},
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
    },
    solver::{
        heuristic::{manhattan::ManhattanDistance, Heuristic as _},
        solver::Solver,
    },
};

use crate::{
    args::Args,
    command::Command,
    enums::{ColoringType, LabelType, Metric, StateFormatter},
    state::State,
    util::{loop_func, try_func, try_func_once},
};

pub struct Runner {
    state: State,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    fn apply(&self, state: &mut Puzzle, alg: &Algorithm) {
        if state.try_apply_alg(alg) {
            println!("{state}");
        } else {
            println!("Invalid");
        }
    }

    fn apply_to_solved(&self, alg: &Algorithm, size: Size) -> Result<(), Box<dyn Error>> {
        let mut state = Puzzle::new(size);
        self.apply(&mut state, alg);

        Ok(())
    }

    fn concat(&self, alg: &Algorithm, prefix: &Algorithm, suffix: &Algorithm) {
        println!("{prefix}{alg}{suffix}");
    }

    fn embed(&self, state: &Puzzle, target: &mut Puzzle) {
        if state.try_embed_into(target) {
            println!("{target}");
        } else {
            println!("Invalid");
        }
    }

    fn filter_optimal(&self, alg: &Algorithm, size: Size, keep_suboptimal: bool) {
        let mut p = Puzzle::new(size);
        let inverse = alg.inverse();

        if !p.try_apply_alg(&inverse) {
            return;
        }

        let solution = self.state.solve(&p);

        let alg_len = alg.len_stm::<u64>();
        let opt_len = solution.len_stm::<u64>();

        if (alg_len == opt_len) ^ keep_suboptimal {
            println!("{alg}");
        }
    }

    fn format(&self, alg: &Algorithm, long: bool, spaced: bool) {
        let s = match (long, spaced) {
            (true, true) => alg.display_long_spaced().to_string(),
            (true, false) => alg.display_long_unspaced().to_string(),
            (false, true) => alg.display_short_spaced().to_string(),
            (false, false) => alg.display_short_unspaced().to_string(),
        };
        println!("{s}");
    }

    fn format_state(&self, state: &Puzzle, formatter: StateFormatter) {
        match formatter {
            StateFormatter::Inline => println!("{}", state.display_inline()),
            StateFormatter::Grid => println!("{}", state.display_grid()),
        }
    }

    fn from_solution(&self, alg: &Algorithm, size: Size) {
        let mut p = Puzzle::new(size);
        if p.try_apply_alg(&alg.inverse()) {
            println!("{p}");
        } else {
            println!("Invalid");
        }
    }

    fn generate(&self, number: u64, size: Size, s: &impl Scrambler) -> Result<(), Box<dyn Error>> {
        let mut p = Puzzle::new(size);

        for _ in 0..number {
            p.reset();
            s.scramble(&mut p);
            println!("{p}");
        }

        Ok(())
    }

    fn invert(&self, alg: &mut Algorithm) {
        alg.invert();
        println!("{alg}");
    }

    fn length(&self, alg: &Algorithm, metric: Metric) {
        let len: u64 = match metric {
            Metric::Stm => alg.len_stm(),
            Metric::Mtm => alg.len_mtm(),
        };
        println!("{len}");
    }

    fn md(&self, state: &Puzzle) {
        if state.is_solvable() {
            let b: u64 = ManhattanDistance(&RowGrids).bound(state);
            println!("{b}");
        } else {
            println!("Unsolvable");
        }
    }

    fn opt_diff(&self, alg: &Algorithm, size: Size) {
        let mut p = Puzzle::new(size);
        p.apply_alg(&alg.inverse());

        let solution = self.state.solve(&p);

        let alg_len = alg.len_stm::<u64>();
        let opt_len = solution.len_stm::<u64>();

        println!("{}", alg_len - opt_len);
    }

    fn optimize(&self, alg: &mut Algorithm, length: u64) -> Result<(), Box<dyn Error>> {
        let mut idx = 0;
        while idx + length <= alg.len_stm() {
            let slice = alg.try_slice(idx..idx + length)?;
            let Some(size) = slice.min_applicable_size() else {
                idx += 1;
                continue;
            };
            let mut puzzle = Puzzle::new(size);
            puzzle.apply_alg(&slice);

            let solution = self.state.solve(&puzzle);

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
        &self,
        state: &Puzzle,
        label_type: LabelType,
        coloring_type: ColoringType,
        tile_size: f32,
        tile_gap: f32,
        border_label: LabelType,
        border_coloring: ColoringType,
        border_thickness: f32,
        output: &str,
    ) -> Result<(), Box<dyn Error>> {
        let grid_size = {
            let (width, height) = state.size().into();
            (width.div_ceil(2), height.div_ceil(2))
        };

        let label = label_type.to_box_dyn_label(Some(grid_size)).unwrap();
        let coloring = coloring_type.to_box_dyn_coloring();

        let base_scheme = Box::new(Scheme::new(label, &coloring));
        let subscheme = if label_type == LabelType::Grids {
            let grid_size = Size::new(grid_size.0, grid_size.1)?;

            Some(Box::new(Tiled::new(
                Scheme::new(SplitFringe, &coloring),
                grid_size,
            )))
        } else {
            None
        };

        let border_label = border_label.to_box_dyn_label(Some(grid_size)).unwrap();
        let border_coloring = border_coloring.to_box_dyn_coloring();
        let border_scheme =
            Box::new(Scheme::new(border_label, &border_coloring)) as Box<dyn ColorScheme>;

        let mut renderer: RendererBuilder<_, _, _> = RendererBuilder::with_scheme(&base_scheme)
            .text(Text::default().font_size(tile_size * 30.0 / 75.0))
            .borders(Borders::with_scheme(border_scheme).thickness(border_thickness))
            .tile_size(tile_size)
            .tile_gap(tile_gap);

        if let Some(subscheme) = subscheme {
            renderer = renderer.subscheme(subscheme);
        }

        let renderer = renderer.build();

        let svg = renderer.render(state)?;
        svg::save(output, &svg)?;

        Ok(())
    }

    fn simplify(&self, alg: &mut Algorithm, verbose: bool) {
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

    fn slice(&self, alg: &Algorithm, start: u64, end: Option<u64>) -> Result<(), Box<dyn Error>> {
        let end = end.unwrap_or_else(|| alg.len_stm());
        let slice = alg.try_slice(start..end)?;
        println!("{slice}");

        Ok(())
    }

    fn solvable(&self, state: &Puzzle) {
        println!("{}", state.is_solvable());
    }

    fn solve(&self, state: &Puzzle, label: LabelType, verbose: bool) -> Result<(), Box<dyn Error>> {
        let a = match label {
            LabelType::Trivial => {
                let mut s = Solver::new(&ManhattanDistance(&Trivial), &Trivial);
                s.solve(state)?
            }
            LabelType::RowGrids => self.state.solve(state),
            LabelType::Rows => {
                let mut s = Solver::new(&ManhattanDistance(&Rows), &Rows);
                s.solve(state)?
            }
            LabelType::Fringe => {
                let mut s = Solver::new(&ManhattanDistance(&Fringe), &Fringe);
                s.solve(state)?
            }
            LabelType::SquareFringe => {
                let mut s = Solver::new(&ManhattanDistance(&SquareFringe), &SquareFringe);
                s.solve(state)?
            }
            LabelType::SplitFringe => {
                let mut s = Solver::new(&ManhattanDistance(&SplitFringe), &SplitFringe);
                s.solve(state)?
            }
            LabelType::SplitSquareFringe => {
                let mut s = Solver::new(&ManhattanDistance(&SplitSquareFringe), &SplitSquareFringe);
                s.solve(state)?
            }
            LabelType::Diagonals => {
                let mut s = Solver::new(&ManhattanDistance(&Diagonals), &Diagonals);
                s.solve(state)?
            }
            LabelType::Checkerboard => {
                let mut s = Solver::new(&ManhattanDistance(&Checkerboard), &Checkerboard);
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

    pub fn run(&self, args: Args) -> Result<(), Box<dyn Error>> {
        match args.command {
            Command::Apply { state, alg } => match (state, alg) {
                (None, None) => unreachable!(),
                (None, Some(alg)) => loop_func(|s| self.apply(s, &alg)),
                (Some(state), None) => loop_func(|a| self.apply(&mut state.clone(), a)),
                (Some(mut state), Some(alg)) => {
                    self.apply(&mut state, &alg);
                    Ok(())
                }
            },
            Command::ApplyToSolved { alg, size } => {
                try_func(|a| self.apply_to_solved(a, size), alg)
            }
            Command::Concat {
                alg,
                prefix,
                suffix,
            } => try_func(|a| self.concat(a, &prefix, &suffix), alg),
            Command::Embed {
                state,
                target,
                size,
            } => {
                let target = size.map(Puzzle::new).or(target);

                match (state, target) {
                    (None, None) => unreachable!(),
                    (None, Some(target)) => loop_func(|s| self.embed(s, &mut target.clone())),
                    (Some(state), None) => loop_func(|t| self.embed(&state.clone(), t)),
                    (Some(state), Some(mut target)) => {
                        self.embed(&state, &mut target);
                        Ok(())
                    }
                }
            }
            Command::FilterOptimal {
                alg,
                size,
                keep_suboptimal,
            } => try_func(|a| self.filter_optimal(a, size, keep_suboptimal), alg),
            Command::Format { alg, long, spaced } => {
                try_func(|a| self.format(a, long, spaced), alg)
            }
            Command::FormatState { state, format } => {
                try_func(|s| self.format_state(s, format), state)
            }
            Command::FromSolution { alg, size } => try_func(|a| self.from_solution(a, size), alg),
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
                    self.generate(
                        number,
                        size,
                        &RandomMoves {
                            moves: num_moves,
                            allow_backtracking,
                            allow_illegal_moves,
                        },
                    )
                } else {
                    self.generate(number, size, &RandomState)
                }
            }
            Command::Invert { alg } => try_func(|a| self.invert(a), alg),
            Command::Length { alg, metric } => try_func(|a| self.length(a, metric), alg),
            Command::Md { state } => try_func(|s| self.md(s), state),
            Command::OptDiff { alg, size } => try_func(|a| self.opt_diff(a, size), alg),
            Command::Optimize { alg, length } => try_func(|a| self.optimize(a, length), alg),
            Command::Render {
                state,
                label,
                coloring,
                tile_size,
                tile_gap,
                border_label,
                border_coloring,
                border_thickness,
                output,
            } => try_func_once(
                |s| {
                    self.render(
                        s,
                        label,
                        coloring,
                        tile_size,
                        tile_gap,
                        border_label,
                        border_coloring,
                        border_thickness,
                        &output,
                    )
                },
                state,
            ),
            Command::Simplify { alg, verbose } => try_func(|a| self.simplify(a, verbose), alg),
            Command::Slice { alg, start, end } => try_func(|a| self.slice(a, start, end), alg),
            Command::Solvable { state } => try_func(|s| self.solvable(s), state),
            Command::Solve {
                state,
                label,
                verbose,
            } => try_func(|s| self.solve(s, label, verbose), state),
        }
    }
}
