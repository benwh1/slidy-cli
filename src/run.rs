use std::{error::Error, ops::ControlFlow};

use rand::{rngs::Xoshiro256PlusPlus, Rng, SeedableRng};
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        color_scheme::{ColorScheme, Scheme},
        label::label::RowGrids,
        puzzle::Puzzle,
        render::{Borders, RendererBuilder, Text},
        scrambler::{RandomMoves, RandomState, Scrambler},
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
    },
    solver::{
        config::SolverConfig,
        heuristic::{manhattan::ManhattanDistance, Heuristic as _},
        solver::Solver as _,
    },
};

use crate::{
    algorithm_ext::AlgorithmExt as _,
    args::Args,
    command::Command,
    enums::{ColoringType, LabelType, Metric, StateFormatter},
    solver::{Solver, SolverContext},
    util::{loop_fn, try_fallible_fn, try_fallible_fn_once, try_fn},
};

pub struct Runner {
    solver: Solver,
}

type Result = core::result::Result<(), Box<dyn Error>>;

impl Runner {
    pub fn new() -> Self {
        Self {
            solver: Solver::new(),
        }
    }

    fn apply(state: &mut Puzzle, alg: &Algorithm) -> Result {
        if state.try_apply_alg(alg) {
            println!("{state}");
            Ok(())
        } else {
            Err("apply: failed to apply algorithm")?
        }
    }

    fn apply_to_solved(alg: &Algorithm, size: Size) -> Result {
        let mut state = Puzzle::new(size);
        Self::apply(&mut state, alg)
    }

    fn concat(alg: &Algorithm, prefix: &Algorithm, suffix: &Algorithm) {
        println!("{prefix}{alg}{suffix}");
    }

    fn embed(state: &Puzzle, target: &mut Puzzle) -> Result {
        if state.try_embed_into(target) {
            println!("{target}");
            Ok(())
        } else {
            Err("embed: failed to embed state")?
        }
    }

    fn filter_optimal(
        &mut self,
        alg: &Algorithm,
        size: Size,
        metric: Metric,
        keep_suboptimal: bool,
    ) -> Result {
        let mut p = Puzzle::new(size);
        let inverse = alg.inverse();

        if !p.try_apply_alg(&inverse) {
            Err("filter-optimal: failed to apply inverse algorithm")?;
        }

        let solution = self.solver.solve_with_context(
            &p,
            &SolverContext {
                metric,
                label: LabelType::RowGrids,
            },
        )?;

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        if (alg_len == opt_len) ^ keep_suboptimal {
            println!("{alg}");
        }

        Ok(())
    }

    fn filter_solvable(state: &Puzzle, keep_unsolvable: bool) {
        if state.is_solvable() ^ keep_unsolvable {
            println!("{state}");
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

    fn from_solution(alg: &Algorithm, size: Size) -> Result {
        let mut p = Puzzle::new(size);

        if p.try_apply_alg(&alg.inverse()) {
            println!("{p}");
            Ok(())
        } else {
            Err("from-solution: failed to apply inverse solution")?
        }
    }

    fn generate(number: u64, size: Size, s: &impl Scrambler, rng: &mut impl Rng) {
        let mut p = Puzzle::new(size);

        for _ in 0..number {
            p.reset();
            s.scramble_with_rng(&mut p, rng);
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
        let md = ManhattanDistance(RowGrids).bound(state);
        println!("{md}");
    }

    fn opt_diff(&mut self, alg: &Algorithm, metric: Metric, size: Size) -> Result {
        let mut p = Puzzle::new(size);

        if !p.try_apply_alg(&alg.inverse()) {
            Err("opt-diff: failed to apply inverse algorithm")?;
        }

        let solution = self.solver.solve_with_context(
            &p,
            &SolverContext {
                metric,
                label: LabelType::RowGrids,
            },
        )?;

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        println!("{}", alg_len - opt_len);

        Ok(())
    }

    fn optimize(&mut self, alg: &mut Algorithm, metric: Metric, length: u64) -> Result {
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

            let solution = self.solver.solve_with_context(
                &puzzle,
                &SolverContext {
                    metric,
                    label: LabelType::RowGrids,
                },
            )?;

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

    fn piece_at(state: &Puzzle, position: u64) -> Result {
        println!(
            "{}",
            state
                .try_piece_at(position)
                .ok_or("piece-at: position out of bounds")?,
        );

        Ok(())
    }

    fn piece_position(state: &Puzzle, piece: u64) -> Result {
        println!(
            "{}",
            state
                .try_piece_position(piece)
                .ok_or("piece-position: piece out of bounds")?,
        );

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
    ) -> Result {
        let coloring = coloring_type.to_box_dyn_coloring();
        let base_scheme = Box::new(Scheme::new(label_type, &coloring));
        let border_coloring = border_coloring.to_box_dyn_coloring();
        let border_scheme =
            Box::new(Scheme::new(border_label, &border_coloring)) as Box<dyn ColorScheme>;

        let renderer: RendererBuilder<_, Box<dyn ColorScheme>, _> =
            RendererBuilder::with_scheme(&base_scheme)
                .text(Text::default().font_size(font_size))
                .borders(Borders::with_scheme(border_scheme).thickness(border_thickness))
                .tile_size(tile_size)
                .tile_gap(tile_gap);

        let renderer = renderer.build();

        let svg = renderer.render(state);
        svg::save(output, &svg)?;

        Ok(())
    }

    fn simplify(alg: &mut Algorithm, verbose: bool) {
        let orig = alg.len_stm();
        alg.simplify();
        let new = alg.len_stm();

        println!("{alg}");
        if verbose {
            println!("Original length: {orig}");

            let diff = orig - new;
            let percent = if orig == 0 {
                0.0
            } else {
                diff as f32 * 100.0 / orig as f32
            };

            println!("New length: {new} [-{diff}, -{percent:.4}%]");
        }
    }

    fn slice(alg: &Algorithm, start: u64, end: Option<u64>, metric: Metric) -> Result {
        let end = end.unwrap_or_else(|| alg.len_metric(metric));
        let slice = alg.slice_metric(metric, start..end)?;
        println!("{slice}");

        Ok(())
    }

    fn solvable(state: &Puzzle) {
        println!("{}", state.is_solvable());
    }

    fn solve(
        &mut self,
        puzzle: &Puzzle,
        metric: Metric,
        label: LabelType,
        config: SolverConfig,
    ) -> Result {
        let context = SolverContext { metric, label };

        Ok(self
            .solver
            .solve_with_config_and_context(puzzle, config, &context)?)
    }

    fn solved_state(size: Size) {
        println!("{}", Puzzle::new(size));
    }

    fn transpose(alg: &Algorithm) {
        println!("{}", alg.transpose());
    }

    pub fn run(&mut self, args: Args) -> Result {
        match args.command {
            Command::Apply { state, alg } => match (state, alg) {
                (None, None) => unreachable!(),
                (None, Some(alg)) => loop_fn(|s| Self::apply(s, &alg)),
                (Some(state), None) => loop_fn(|a| Self::apply(&mut state.clone(), a)),
                (Some(mut state), Some(alg)) => Self::apply(&mut state, &alg),
            },
            Command::ApplyToSolved { alg, size } => {
                try_fallible_fn(|a| Self::apply_to_solved(a, size), alg)
            }
            Command::Concat {
                alg,
                prefix,
                suffix,
            } => try_fn(|a| Self::concat(a, &prefix, &suffix), alg),
            Command::Embed {
                state,
                target,
                size,
            } => {
                let target = size.map(Puzzle::new).or(target);

                match (state, target) {
                    (None, None) => unreachable!(),
                    (None, Some(target)) => loop_fn(|s| Self::embed(s, &mut target.clone())),
                    (Some(state), None) => loop_fn(|t| Self::embed(&state.clone(), t)),
                    (Some(state), Some(mut target)) => Self::embed(&state, &mut target),
                }
            }
            Command::FilterOptimal {
                alg,
                size,
                metric,
                keep_suboptimal,
            } => try_fallible_fn(
                |a| self.filter_optimal(a, size, metric, keep_suboptimal),
                alg,
            ),
            Command::FilterSolvable { state, unsolvable } => {
                try_fn(|s| Self::filter_solvable(s, unsolvable), state)
            }
            Command::Format { alg, long, spaced } => try_fn(|a| Self::format(a, long, spaced), alg),
            Command::FormatState { state, format } => {
                try_fn(|s| Self::format_state(s, format), state)
            }
            Command::FromSolution { alg, size } => {
                try_fallible_fn(|a| Self::from_solution(a, size), alg)
            }
            Command::Generate {
                number,
                size,
                seed,
                random_moves,
                num_moves,
                allow_backtracking,
                allow_illegal_moves,
                ..
            } => {
                if random_moves {
                    let scrambler = RandomMoves {
                        moves: num_moves,
                        allow_backtracking,
                        allow_illegal_moves,
                    };
                    match seed {
                        Some(seed) => Self::generate(
                            number,
                            size,
                            &scrambler,
                            &mut Xoshiro256PlusPlus::seed_from_u64(seed),
                        ),
                        None => Self::generate(number, size, &scrambler, &mut rand::rng()),
                    };
                } else {
                    match seed {
                        Some(seed) => Self::generate(
                            number,
                            size,
                            &RandomState,
                            &mut Xoshiro256PlusPlus::seed_from_u64(seed),
                        ),
                        None => Self::generate(number, size, &RandomState, &mut rand::rng()),
                    };
                }

                Ok(())
            }
            Command::Invert { alg } => try_fn(Self::invert, alg),
            Command::Length { alg, metric } => try_fn(|a| Self::length(a, metric), alg),
            Command::Md { state } => try_fn(|s| Self::md(s), state),
            Command::OptDiff { alg, size, metric } => {
                try_fallible_fn(|a| self.opt_diff(a, metric, size), alg)
            }
            Command::Optimize {
                alg,
                metric,
                length,
            } => try_fallible_fn(|a| self.optimize(a, metric, length), alg),
            Command::PieceAt { state, position } => {
                try_fallible_fn(|s| Self::piece_at(s, position), state)
            }
            Command::PiecePosition { state, piece } => {
                try_fallible_fn(|s| Self::piece_position(s, piece), state)
            }
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
            } => try_fallible_fn_once(
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
            Command::Simplify { alg, verbose } => try_fn(|a| Self::simplify(a, verbose), alg),
            Command::Slice {
                alg,
                start,
                end,
                metric,
            } => try_fallible_fn(|a| Self::slice(a, start, end, metric), alg),
            Command::Solvable { state } => try_fn(|s| Self::solvable(s), state),
            Command::Solve {
                state,
                metric,
                label,
                num_solutions,
                min_depth,
                max_depth,
                depth_beyond_optimal,
                show_progress,
                verbose,
                ..
            } => try_fallible_fn(
                |s| {
                    let config = SolverConfig {
                        min: min_depth,
                        max: max_depth,
                        depth_beyond_optimal,
                        num_solutions,
                        end_of_iter_callback: show_progress.then_some(Box::new(
                            |stats| -> ControlFlow<()> {
                                let depth = stats.depth;
                                eprintln!("Finished searching depth {depth}");
                                ControlFlow::Continue(())
                            },
                        )),
                        solution_callback: Some(Box::new(move |alg| -> ControlFlow<()> {
                            println!("{alg}");
                            if verbose {
                                println!("{} moves", alg.len_metric(metric));
                            }
                            ControlFlow::Continue(())
                        })),
                    };

                    self.solve(s, metric, label, config)
                },
                state,
            ),
            Command::SolvedState { size } => try_fn(|s| Self::solved_state(*s), size),
            Command::Transpose { alg } => try_fn(|a| Self::transpose(a), alg),
        }
    }
}
