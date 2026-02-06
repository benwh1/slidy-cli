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
    algorithm_ext::AlgorithmExt as _,
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

    fn apply(state: &mut Puzzle, alg: &Algorithm) {
        if state.try_apply_alg(alg) {
            println!("{state}");
        } else {
            println!("Invalid");
        }
    }

    fn apply_to_solved(alg: &Algorithm, size: Size) {
        let mut state = Puzzle::new(size);
        Self::apply(&mut state, alg);
    }

    fn concat(alg: &Algorithm, prefix: &Algorithm, suffix: &Algorithm) {
        println!("{prefix}{alg}{suffix}");
    }

    fn embed(state: &Puzzle, target: &mut Puzzle) {
        if state.try_embed_into(target) {
            println!("{target}");
        } else {
            println!("Invalid");
        }
    }

    fn filter_optimal(&self, alg: &Algorithm, size: Size, metric: Metric, keep_suboptimal: bool) {
        let mut p = Puzzle::new(size);
        let inverse = alg.inverse();

        if !p.try_apply_alg(&inverse) {
            return;
        }

        let solution = self.state.solve(&p, metric);

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        if (alg_len == opt_len) ^ keep_suboptimal {
            println!("{alg}");
        }
    }

    fn format(alg: &Algorithm, long: bool, spaced: bool) {
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

    fn from_solution(alg: &Algorithm, size: Size) {
        let mut p = Puzzle::new(size);
        if p.try_apply_alg(&alg.inverse()) {
            println!("{p}");
        } else {
            println!("Invalid");
        }
    }

    fn generate(number: u64, size: Size, s: &impl Scrambler) {
        let mut p = Puzzle::new(size);

        for _ in 0..number {
            p.reset();
            s.scramble(&mut p);
            println!("{p}");
        }
    }

    fn invert(alg: &mut Algorithm) {
        alg.invert();
        println!("{alg}");
    }

    fn length(alg: &Algorithm, metric: Metric) {
        let len = alg.len_metric(metric);
        println!("{len}");
    }

    fn md(state: &Puzzle) {
        if state.is_solvable() {
            let b: u64 = ManhattanDistance(&RowGrids).bound(state);
            println!("{b}");
        } else {
            println!("Unsolvable");
        }
    }

    fn opt_diff(&self, alg: &Algorithm, metric: Metric, size: Size) {
        let mut p = Puzzle::new(size);
        p.apply_alg(&alg.inverse());

        let solution = self.state.solve(&p, metric);

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        println!("{}", alg_len - opt_len);
    }

    fn optimize(
        &self,
        alg: &mut Algorithm,
        metric: Metric,
        length: u64,
    ) -> Result<(), Box<dyn Error>> {
        alg.simplify();

        let mut idx = 0;
        while idx + length <= alg.len_metric(metric) {
            let slice = alg.slice_metric(metric, idx..idx + length)?;
            let Some(size) = slice.min_applicable_size() else {
                idx += 1;
                continue;
            };
            let mut puzzle = Puzzle::new(size);
            puzzle.apply_alg(&slice);

            let solution = self.state.solve(&puzzle, metric);

            if solution.len_metric(metric) == length {
                idx += 1;
            } else {
                let mut start = Algorithm::from(alg.slice_metric(metric, 0..idx)?);
                let middle = solution.inverse();
                let end = Algorithm::from(
                    alg.slice_metric(metric, idx + length..alg.len_metric(metric))?,
                );
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
        tile_gap: f32,
        border_label: LabelType,
        border_coloring: ColoringType,
        border_thickness: f32,
        font_size: f32,
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
            .text(Text::default().font_size(font_size))
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

    fn slice(alg: &Algorithm, start: u64, end: Option<u64>) -> Result<(), Box<dyn Error>> {
        let end = end.unwrap_or_else(|| alg.len_stm());
        let slice = alg.try_slice(start..end)?;
        println!("{slice}");

        Ok(())
    }

    fn solvable(state: &Puzzle) {
        println!("{}", state.is_solvable());
    }

    fn solve(
        &self,
        state: &Puzzle,
        metric: Metric,
        label: LabelType,
        verbose: bool,
    ) -> Result<(), Box<dyn Error>> {
        if metric == Metric::Mtm && label != LabelType::RowGrids {
            return Err("solving labels other than row grids in MTM is not supported".into());
        }

        let a = match label {
            LabelType::Trivial => {
                let mut s = Solver::new(&ManhattanDistance(&Trivial), &Trivial);
                s.solve(state)?
            }
            LabelType::RowGrids => self.state.solve(state, metric),
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
            println!("{} moves", a.len_metric(metric));
        }

        Ok(())
    }

    pub fn run(&self, args: Args) -> Result<(), Box<dyn Error>> {
        match args.command {
            Command::Apply { state, alg } => match (state, alg) {
                (None, None) => unreachable!(),
                (None, Some(alg)) => loop_func(|s| Self::apply(s, &alg)),
                (Some(state), None) => loop_func(|a| Self::apply(&mut state.clone(), a)),
                (Some(mut state), Some(alg)) => {
                    Self::apply(&mut state, &alg);
                    Ok(())
                }
            },
            Command::ApplyToSolved { alg, size } => {
                try_func(|a| Self::apply_to_solved(a, size), alg)
            }
            Command::Concat {
                alg,
                prefix,
                suffix,
            } => try_func(|a| Self::concat(a, &prefix, &suffix), alg),
            Command::Embed {
                state,
                target,
                size,
            } => {
                let target = size.map(Puzzle::new).or(target);

                match (state, target) {
                    (None, None) => unreachable!(),
                    (None, Some(target)) => loop_func(|s| Self::embed(s, &mut target.clone())),
                    (Some(state), None) => loop_func(|t| Self::embed(&state.clone(), t)),
                    (Some(state), Some(mut target)) => {
                        Self::embed(&state, &mut target);
                        Ok(())
                    }
                }
            }
            Command::FilterOptimal {
                alg,
                size,
                metric,
                keep_suboptimal,
            } => try_func(
                |a| self.filter_optimal(a, size, metric, keep_suboptimal),
                alg,
            ),
            Command::Format { alg, long, spaced } => {
                try_func(|a| Self::format(a, long, spaced), alg)
            }
            Command::FormatState { state, format } => {
                try_func(|s| Self::format_state(s, format), state)
            }
            Command::FromSolution { alg, size } => try_func(|a| Self::from_solution(a, size), alg),
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
                    Self::generate(
                        number,
                        size,
                        &RandomMoves {
                            moves: num_moves,
                            allow_backtracking,
                            allow_illegal_moves,
                        },
                    );
                } else {
                    Self::generate(number, size, &RandomState);
                }

                Ok(())
            }
            Command::Invert { alg } => try_func(Self::invert, alg),
            Command::Length { alg, metric } => try_func(|a| Self::length(a, metric), alg),
            Command::Md { state } => try_func(|s| Self::md(s), state),
            Command::OptDiff { alg, size, metric } => {
                try_func(|a| self.opt_diff(a, metric, size), alg)
            }
            Command::Optimize {
                alg,
                metric,
                length,
            } => try_func(|a| self.optimize(a, metric, length), alg),
            Command::Render {
                state,
                label,
                coloring,
                tile_size,
                tile_gap,
                border_label,
                border_coloring,
                border_thickness,
                font_size,
                output,
            } => try_func_once(
                |s| {
                    Self::render(
                        s,
                        label,
                        coloring,
                        tile_size,
                        tile_gap,
                        border_label,
                        border_coloring,
                        border_thickness,
                        font_size,
                        &output,
                    )
                },
                state,
            ),
            Command::Simplify { alg, verbose } => try_func(|a| Self::simplify(a, verbose), alg),
            Command::Slice { alg, start, end } => try_func(|a| Self::slice(a, start, end), alg),
            Command::Solvable { state } => try_func(|s| Self::solvable(s), state),
            Command::Solve {
                state,
                metric,
                label,
                verbose,
            } => try_func(|s| self.solve(s, metric, label, verbose), state),
        }
    }
}
