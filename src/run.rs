use std::{
    error::Error,
    io::{self, BufWriter, IsTerminal as _, Write},
    ops::ControlFlow,
};

use clap::CommandFactory as _;
use clap_complete::aot::{self, Shell};
use rand::{rngs::Xoshiro256PlusPlus, Rng, SeedableRng as _};
use slidy::{
    algorithm::{algorithm::Algorithm, r#move::r#move::Move},
    puzzle::{
        color_scheme::{ColorScheme, Scheme},
        label::label::RowGrids,
        puzzle::Puzzle,
        render::{Borders, RendererBuilder, Text},
        scrambler::{RandomInvertibleState, RandomMoves, RandomState, Scrambler},
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
    command::{Command, PdbOptions, RenderOptions},
    enums::{LabelType, Metric, StateFormatter},
    solver::{Solver, SolverContext},
    util::{loop_fn, try_fallible_fn, try_fallible_fn_once},
};

/// Length, in `metric`, of the longest common prefix of moves of `a` and `b`.
fn unchanged_prefix_len(a: &Algorithm, b: &Algorithm, metric: Metric) -> u64 {
    let common = a
        .moves()
        .iter()
        .zip(b.moves())
        .take_while(|(x, y)| x == y)
        .count();

    Algorithm::with_moves(b.moves()[..common].to_vec()).len_metric(metric)
}

pub struct Runner {
    /// Created on first use, so that commands which don't solve anything never touch the PDB
    /// cache.
    solver: Option<Solver>,

    /// The options the solver is created with: which 4x4 STM solver to use, and whether to print
    /// progress while building pattern databases.
    pdb_options: PdbOptions,

    /// Whether `apply --trace` has printed a trace yet, so that the next one is preceded by a
    /// blank line.
    traced: bool,

    /// Where command output goes. Buffered unless stdout is a terminal, so that large streams
    /// don't cost a `write` syscall per line.
    out: Box<dyn Write>,
}

type Result = core::result::Result<(), Box<dyn Error>>;

impl Runner {
    pub fn new() -> Self {
        let stdout = io::stdout();
        let out: Box<dyn Write> = if stdout.is_terminal() {
            Box::new(stdout)
        } else {
            Box::new(BufWriter::new(stdout))
        };

        Self {
            solver: None,
            pdb_options: PdbOptions::default(),
            traced: false,
            out,
        }
    }

    fn solver(&mut self) -> &mut Solver {
        let pdb_options = self.pdb_options;
        self.solver.get_or_insert_with(|| Solver::new(pdb_options))
    }

    fn apply(&mut self, state: &mut Puzzle, alg: &Algorithm) -> Result {
        if state.try_apply_alg(alg) {
            writeln!(self.out, "{state}")?;
            Ok(())
        } else {
            Err("failed to apply algorithm")?
        }
    }

    /// Prints `state`, then applies the moves of `alg` (counted in `metric`) one at a time,
    /// printing the state after each. If a move can't be made, the states before it have already
    /// been printed.
    fn apply_traced(&mut self, state: &mut Puzzle, alg: &Algorithm, metric: Metric) -> Result {
        if self.traced {
            writeln!(self.out)?;
        }
        self.traced = true;

        writeln!(self.out, "{state}")?;

        let moves: Box<dyn Iterator<Item = Move>> = match metric {
            Metric::Stm => Box::new(alg.moves_stm().map(|dir| Move::new(dir, 1))),
            Metric::Mtm => Box::new(alg.moves_mtm()),
        };

        for (i, mv) in moves.enumerate() {
            if !state.try_apply_move(mv) {
                Err(format!(
                    "failed to apply algorithm: move {} ({mv}) can't be made",
                    i + 1
                ))?;
            }
            writeln!(self.out, "{state}")?;
        }

        Ok(())
    }

    fn apply_to_solved(&mut self, alg: &Algorithm, size: Size) -> Result {
        let mut state = Puzzle::new(size);
        self.apply(&mut state, alg)
    }

    fn completions(&mut self, shell: Shell) {
        let mut cmd = Args::command();
        aot::generate(shell, &mut cmd, "slidy", &mut self.out);
    }

    fn concat(&mut self, alg: &Algorithm, prefix: &Algorithm, suffix: &Algorithm) -> Result {
        writeln!(self.out, "{prefix}{alg}{suffix}")?;
        Ok(())
    }

    fn embed(&mut self, state: &Puzzle, target: &mut Puzzle) -> Result {
        if state.try_embed_into(target) {
            writeln!(self.out, "{target}")?;
            Ok(())
        } else {
            Err("failed to embed state")?
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
            Err("failed to apply inverse algorithm")?;
        }

        let solution = self.solver().solve_with_context(
            &p,
            &SolverContext {
                metric,
                label: LabelType::RowGrids,
            },
        )?;

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        if (alg_len == opt_len) ^ keep_suboptimal {
            writeln!(self.out, "{alg}")?;
        }

        // Solving can be slow, so show each result as soon as it's ready.
        self.out.flush()?;

        Ok(())
    }

    fn filter_solvable(&mut self, state: &Puzzle, keep_unsolvable: bool) -> Result {
        if state.is_solvable() ^ keep_unsolvable {
            writeln!(self.out, "{state}")?;
        }

        Ok(())
    }

    fn format(&mut self, alg: &Algorithm, long: bool, spaced: bool) -> Result {
        match (long, spaced) {
            (true, true) => writeln!(self.out, "{}", alg.display_long_spaced())?,
            (true, false) => writeln!(self.out, "{}", alg.display_long_unspaced())?,
            (false, true) => writeln!(self.out, "{}", alg.display_short_spaced())?,
            (false, false) => writeln!(self.out, "{}", alg.display_short_unspaced())?,
        }

        Ok(())
    }

    fn format_state(&mut self, state: &Puzzle, formatter: StateFormatter) -> Result {
        match formatter {
            StateFormatter::Inline => writeln!(self.out, "{}", state.display_inline())?,
            StateFormatter::Grid => writeln!(self.out, "{}", state.display_grid())?,
        }

        Ok(())
    }

    fn state_from_solution(&mut self, alg: &Algorithm, size: Size) -> Result {
        let mut p = Puzzle::new(size);

        if p.try_apply_alg(&alg.inverse()) {
            writeln!(self.out, "{p}")?;
            Ok(())
        } else {
            Err("failed to apply inverse solution")?
        }
    }

    /// Generates `number` scrambles with `s`, using a seeded RNG if `seed` is given.
    fn generate(
        &mut self,
        number: u64,
        size: Size,
        s: &impl Scrambler,
        seed: Option<u64>,
    ) -> Result {
        match seed {
            Some(seed) => self.generate_with_rng(
                number,
                size,
                s,
                &mut Xoshiro256PlusPlus::seed_from_u64(seed),
            ),
            None => self.generate_with_rng(number, size, s, &mut rand::rng()),
        }
    }

    fn generate_with_rng(
        &mut self,
        number: u64,
        size: Size,
        s: &impl Scrambler,
        rng: &mut impl Rng,
    ) -> Result {
        // Scrambling a puzzle of the wrong size panics, e.g. random moves without backtracking on
        // a 1xN puzzle, or an invertible state on a 1x1 puzzle.
        if !s.is_valid_size(size) {
            Err(format!(
                "this kind of scramble can't be generated for a {size} puzzle"
            ))?;
        }

        let mut p = Puzzle::new(size);

        for _ in 0..number {
            p.reset();
            s.scramble_with_rng(&mut p, rng);
            writeln!(self.out, "{p}")?;
        }

        Ok(())
    }

    fn invert(&mut self, alg: &mut Algorithm) -> Result {
        alg.invert();
        writeln!(self.out, "{alg}")?;
        Ok(())
    }

    fn length(&mut self, alg: &Algorithm, metric: Metric) -> Result {
        let len = alg.len_metric(metric);
        writeln!(self.out, "{len}")?;
        Ok(())
    }

    fn md(&mut self, state: &Puzzle) -> Result {
        let md = ManhattanDistance(RowGrids).bound(state);
        writeln!(self.out, "{md}")?;
        Ok(())
    }

    fn opt_diff(&mut self, alg: &Algorithm, metric: Metric, size: Size) -> Result {
        let mut p = Puzzle::new(size);

        if !p.try_apply_alg(&alg.inverse()) {
            Err("failed to apply inverse algorithm")?;
        }

        let solution = self.solver().solve_with_context(
            &p,
            &SolverContext {
                metric,
                label: LabelType::RowGrids,
            },
        )?;

        let alg_len = alg.len_metric(metric);
        let opt_len = solution.len_metric(metric);

        writeln!(self.out, "{}", alg_len - opt_len)?;

        // Solving can be slow, so show each result as soon as it's ready.
        self.out.flush()?;

        Ok(())
    }

    fn optimize(&mut self, alg: &mut Algorithm, metric: Metric, length: u64) -> Result {
        self.optimize_alg(alg, metric, length)?;
        writeln!(self.out, "{alg}")?;

        // Solving can be slow, so show each result as soon as it's ready.
        self.out.flush()?;

        Ok(())
    }

    /// Replaces every window of `length` moves in `alg` that has a shorter equivalent with an
    /// optimal one, until no window can be improved.
    fn optimize_alg(&mut self, alg: &mut Algorithm, metric: Metric, length: u64) -> Result {
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

            let solution = self.solver().solve_with_context(
                &puzzle,
                &SolverContext {
                    metric,
                    label: LabelType::RowGrids,
                },
            )?;

            if solution.len_metric(metric) == length {
                idx += 1;
            } else {
                let mut new_alg = Algorithm::from(alg.slice_metric(metric, 0..idx)?);
                new_alg += solution.inverse();
                new_alg += Algorithm::from(
                    alg.slice_metric(metric, idx + length..alg.len_metric(metric))?,
                );

                // The new middle can merge or cancel with the moves around it, which can in turn
                // make earlier windows improvable.
                new_alg.simplify();

                // Windows that lie entirely within the unchanged prefix have already been checked.
                // Every other window from the first changed move onwards needs to be (re)checked.
                let unchanged = unchanged_prefix_len(alg, &new_alg, metric);
                idx = idx.min((unchanged + 1).saturating_sub(length));

                *alg = new_alg;
            }
        }

        Ok(())
    }

    fn piece_at(&mut self, state: &Puzzle, position: u64) -> Result {
        let piece = state
            .try_piece_at(position)
            .ok_or("position out of bounds")?;
        writeln!(self.out, "{piece}")?;

        Ok(())
    }

    fn piece_position(&mut self, state: &Puzzle, piece: u64) -> Result {
        let position = state
            .try_piece_position(piece)
            .ok_or("piece out of bounds")?;
        writeln!(self.out, "{position}")?;

        Ok(())
    }

    fn render(&mut self, state: &Puzzle, options: &RenderOptions) -> Result {
        let RenderOptions {
            label,
            coloring,
            tile_size,
            tile_gap,
            border_label,
            border_coloring,
            border_thickness,
            font_size,
            ref output,
        } = *options;

        let coloring = coloring.to_box_dyn_coloring();
        let base_scheme = Box::new(Scheme::new(label, &coloring));
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
        if output == "-" {
            svg::write(&mut self.out, &svg)?;
            writeln!(self.out)?;
        } else {
            svg::save(output, &svg).map_err(|e| format!("could not write {output}: {e}"))?;
        }

        Ok(())
    }

    fn simplify(&mut self, alg: &mut Algorithm, metric: Metric, verbose: bool) -> Result {
        let orig = alg.len_metric(metric);
        alg.simplify();
        let new = alg.len_metric(metric);

        writeln!(self.out, "{alg}")?;
        if verbose {
            eprintln!("Original length: {orig}");

            let diff = orig - new;
            let percent = if orig == 0 {
                0.0
            } else {
                diff as f32 * 100.0 / orig as f32
            };

            eprintln!("New length: {new} [-{diff}, -{percent:.4}%]");
        }

        Ok(())
    }

    fn slice(&mut self, alg: &Algorithm, start: u64, end: Option<u64>, metric: Metric) -> Result {
        let end = end.unwrap_or_else(|| alg.len_metric(metric));
        let slice = alg
            .slice_metric(metric, start..end)
            .map_err(|e| format!("invalid slice {start}..{end}: {e}"))?;
        writeln!(self.out, "{slice}")?;

        Ok(())
    }

    fn solvable(&mut self, state: &Puzzle) -> Result {
        writeln!(self.out, "{}", state.is_solvable())?;
        Ok(())
    }

    fn solve(
        &mut self,
        puzzle: &Puzzle,
        metric: Metric,
        label: LabelType,
        config: SolverConfig,
    ) -> Result {
        let context = SolverContext { metric, label };

        // Solutions are printed directly to stdout by the solution callback, so anything already
        // written has to come first.
        self.out.flush()?;

        Ok(self
            .solver()
            .solve_with_config_and_context(puzzle, config, &context)?)
    }

    fn solved_state(&mut self, size: Size) -> Result {
        writeln!(self.out, "{}", Puzzle::new(size))?;
        Ok(())
    }

    fn transpose(&mut self, alg: &Algorithm) -> Result {
        writeln!(self.out, "{}", alg.transpose())?;
        Ok(())
    }

    pub fn run(&mut self, args: Args) -> Result {
        let result = self.run_command(args);

        // Flush whatever was written before an error, too.
        let flushed = self.out.flush();
        result?;
        Ok(flushed?)
    }

    fn run_command(&mut self, args: Args) -> Result {
        self.pdb_options = args.command.pdb_options().copied().unwrap_or_default();

        match args.command {
            Command::Apply { state, alg, trace } => {
                let mut apply = |state: &mut Puzzle, alg: &Algorithm| match trace {
                    Some(metric) => self.apply_traced(state, alg, metric),
                    None => self.apply(state, alg),
                };

                match (state, alg) {
                    (None, None) => unreachable!(),
                    (None, Some(alg)) => loop_fn(|s| apply(s, &alg)),
                    (Some(state), None) => loop_fn(|a| apply(&mut state.clone(), a)),
                    (Some(mut state), Some(alg)) => apply(&mut state, &alg),
                }
            }
            Command::ApplyToSolved { alg, size } => {
                try_fallible_fn(|a| self.apply_to_solved(a, size), alg)
            }
            Command::Completions { shell } => {
                self.completions(shell);
                Ok(())
            }
            Command::Concat {
                alg,
                prefix,
                suffix,
            } => try_fallible_fn(|a| self.concat(a, &prefix, &suffix), alg),
            Command::Embed {
                state,
                target,
                size,
            } => {
                let target = size.map(Puzzle::new).or(target);

                match (state, target) {
                    (None, None) => unreachable!(),
                    (None, Some(target)) => loop_fn(|s| self.embed(s, &mut target.clone())),
                    (Some(state), None) => loop_fn(|t| self.embed(&state, t)),
                    (Some(state), Some(mut target)) => self.embed(&state, &mut target),
                }
            }
            Command::FilterOptimal {
                alg,
                size,
                metric,
                keep_suboptimal,
                ..
            } => try_fallible_fn(
                |a| self.filter_optimal(a, size, metric, keep_suboptimal),
                alg,
            ),
            Command::FilterSolvable { state, unsolvable } => {
                try_fallible_fn(|s| self.filter_solvable(s, unsolvable), state)
            }
            Command::Format { alg, long, spaced } => {
                try_fallible_fn(|a| self.format(a, long, spaced), alg)
            }
            Command::FormatState { state, format } => {
                try_fallible_fn(|s| self.format_state(s, format), state)
            }
            Command::FromSolution { alg, size } => {
                try_fallible_fn(|a| self.state_from_solution(a, size), alg)
            }
            Command::Generate {
                number,
                size,
                seed,
                random_moves,
                invertible,
                num_moves,
                allow_backtracking,
                allow_illegal_moves,
            } => {
                if random_moves {
                    let scrambler = RandomMoves {
                        moves: num_moves,
                        allow_backtracking,
                        allow_illegal_moves,
                    };
                    self.generate(number, size, &scrambler, seed)
                } else if invertible {
                    self.generate(number, size, &RandomInvertibleState, seed)
                } else {
                    self.generate(number, size, &RandomState, seed)
                }
            }
            Command::Invert { alg } => try_fallible_fn(|a| self.invert(a), alg),
            Command::Length { alg, metric } => try_fallible_fn(|a| self.length(a, metric), alg),
            Command::Md { state } => try_fallible_fn(|s| self.md(s), state),
            Command::OptDiff {
                alg, size, metric, ..
            } => try_fallible_fn(|a| self.opt_diff(a, metric, size), alg),
            Command::Optimize {
                alg,
                metric,
                length,
                ..
            } => try_fallible_fn(|a| self.optimize(a, metric, length), alg),
            Command::PieceAt { state, position } => {
                try_fallible_fn(|s| self.piece_at(s, position), state)
            }
            Command::PiecePosition { state, piece } => {
                try_fallible_fn(|s| self.piece_position(s, piece), state)
            }
            Command::Render { state, options } => {
                try_fallible_fn_once(|s| self.render(s, &options), state)
            }
            Command::Simplify {
                alg,
                metric,
                verbose,
            } => try_fallible_fn(|a| self.simplify(a, metric, verbose), alg),
            Command::Slice {
                alg,
                start,
                end,
                metric,
            } => try_fallible_fn(|a| self.slice(a, start, end, metric), alg),
            Command::Solvable { state } => try_fallible_fn(|s| self.solvable(s), state),
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
                        max: max_depth.unwrap_or(u64::MAX),
                        depth_beyond_optimal: depth_beyond_optimal.unwrap_or(u64::MAX),
                        num_solutions,
                        end_of_iter_callback: show_progress.then_some(Box::new(
                            |stats| -> ControlFlow<()> {
                                let depth = stats.depth;
                                eprintln!("Finished searching depth {depth}");
                                ControlFlow::Continue(())
                            },
                        )),
                        solution_callback: Some(Box::new(move |alg| -> ControlFlow<()> {
                            // The callback must be `'static`, so it can't use `self.out`. Write
                            // each solution straight to stdout so it appears as soon as it's
                            // found, and stop searching if stdout is gone.
                            let mut out = io::stdout().lock();
                            let written = writeln!(out, "{alg}").and_then(|()| out.flush());

                            if verbose {
                                eprintln!("{} moves", alg.len_metric(metric));
                            }

                            if written.is_ok() {
                                ControlFlow::Continue(())
                            } else {
                                ControlFlow::Break(())
                            }
                        })),
                    };

                    self.solve(s, metric, label, config)
                },
                state,
            ),
            Command::SolvedState { size } => try_fallible_fn(|s| self.solved_state(*s), size),
            Command::Transpose { alg } => try_fallible_fn(|a| self.transpose(a), alg),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, str::FromStr as _};

    use clap::Parser as _;

    use super::*;
    use crate::enums::Pdb4x4Stm;

    fn runner() -> Runner {
        Runner {
            solver: Some(Solver::with_pdb_dir(None, PdbOptions::default())),
            pdb_options: PdbOptions::default(),
            traced: false,
            out: Box::new(io::sink()),
        }
    }

    /// An output buffer that can be read after a [`Runner`] has written to it.
    #[derive(Clone, Default)]
    struct SharedBuf(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl SharedBuf {
        fn lines(&self) -> Vec<String> {
            String::from_utf8(self.0.borrow().clone())
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    /// A [`Runner`] whose output can be read from the returned buffer.
    fn capturing_runner() -> (Runner, SharedBuf) {
        let buf = SharedBuf::default();
        let runner = Runner {
            out: Box::new(buf.clone()),
            ..runner()
        };
        (runner, buf)
    }

    fn trace(state: &str, alg: &str, metric: Metric) -> (Result, Vec<String>) {
        let (mut runner, buf) = capturing_runner();
        let mut state = Puzzle::from_str(state).unwrap();
        let alg = Algorithm::from_str(alg).unwrap();
        let result = runner.apply_traced(&mut state, &alg, metric);
        (result, buf.lines())
    }

    #[test]
    fn apply_trace_prints_every_move() {
        let (result, lines) = trace("1 2 3/4 5 6/7 8 0", "D2R", Metric::Stm);
        result.unwrap();
        assert_eq!(
            lines,
            [
                "1 2 3/4 5 6/7 8 0",
                "1 2 3/4 5 0/7 8 6",
                "1 2 0/4 5 3/7 8 6",
                "1 0 2/4 5 3/7 8 6",
            ]
        );

        // In MTM, D2 is one move.
        let (result, lines) = trace("1 2 3/4 5 6/7 8 0", "D2R", Metric::Mtm);
        result.unwrap();
        assert_eq!(
            lines,
            [
                "1 2 3/4 5 6/7 8 0",
                "1 2 0/4 5 3/7 8 6",
                "1 0 2/4 5 3/7 8 6",
            ]
        );

        let (result, lines) = trace("1 2 3/4 5 6/7 8 0", "", Metric::Stm);
        result.unwrap();
        assert_eq!(lines, ["1 2 3/4 5 6/7 8 0"]);
    }

    #[test]
    fn apply_trace_stops_at_the_first_impossible_move() {
        // The third D would move the gap off the top of the puzzle.
        let (result, lines) = trace("1 2 3/4 5 6/7 8 0", "D3", Metric::Stm);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("move 3 (D)"), "{err}");
        assert_eq!(lines.len(), 3, "{lines:?}");

        let (result, lines) = trace("1 2 3/4 5 6/7 8 0", "D3", Metric::Mtm);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("move 1 (D3)"), "{err}");
        assert_eq!(lines, ["1 2 3/4 5 6/7 8 0"]);
    }

    #[test]
    fn apply_traces_are_separated_by_blank_lines() {
        let (mut runner, buf) = capturing_runner();
        for alg in ["D", "R"] {
            let mut state = Puzzle::from_str("1 2/3 0").unwrap();
            let alg = Algorithm::from_str(alg).unwrap();
            runner.apply_traced(&mut state, &alg, Metric::Stm).unwrap();
        }
        assert_eq!(
            buf.lines(),
            ["1 2/3 0", "1 0/3 2", "", "1 2/3 0", "1 2/0 3"]
        );
    }

    fn generate(size: &str, scrambler: &impl Scrambler, seed: u64) -> (Result, Vec<Puzzle>) {
        let (mut runner, buf) = capturing_runner();
        let size = Size::from_str(size).unwrap();
        let result = runner.generate(20, size, scrambler, Some(seed));
        let states = buf
            .lines()
            .iter()
            .map(|line| Puzzle::from_str(line).unwrap())
            .collect();
        (result, states)
    }

    #[test]
    fn generate_invertible_keeps_the_gap_solved() {
        for size in ["2x2", "3x2", "2x3", "4x4", "5x3"] {
            let (result, states) = generate(size, &RandomInvertibleState, 7);
            result.unwrap();
            assert_eq!(states.len(), 20);

            for state in &states {
                assert!(state.is_solvable(), "{state}");
                assert_eq!(state.gap_position(), state.num_pieces(), "{state}");
            }

            // Not every scramble of a 2x2 can be different from solved.
            if size != "2x2" {
                assert!(states.iter().any(|s| !s.is_solved()), "{size}");
            }
        }

        // The gap can't stay put on a 1xN puzzle.
        let (result, states) = generate("1x3", &RandomInvertibleState, 0);
        assert!(result.is_err());
        assert!(states.is_empty());
    }

    #[test]
    fn generate_rejects_sizes_the_scrambler_cannot_handle() {
        // Without backtracking or illegal moves, random moves run out of moves on a 1xN puzzle,
        // which used to panic.
        let random_moves = RandomMoves {
            moves: 5,
            allow_backtracking: false,
            allow_illegal_moves: false,
        };
        let (result, states) = generate("1x3", &random_moves, 0);
        assert!(result.is_err());
        assert!(states.is_empty());
    }

    #[test]
    fn generate_scrambles_each_state_from_solved() {
        // Each scramble is independent: one random move from solved, not from the last scramble.
        let one_move = RandomMoves {
            moves: 1,
            allow_backtracking: false,
            allow_illegal_moves: false,
        };
        let (result, states) = generate("4x4", &one_move, 3);
        result.unwrap();
        for state in &states {
            assert_eq!(ManhattanDistance(RowGrids).bound(state), 1, "{state}");
        }
    }

    fn parse_args(args: &[&str]) -> core::result::Result<Args, clap::Error> {
        Args::try_parse_from([&["slidy"], args].concat())
    }

    #[test]
    fn cli_solver() {
        let solver = |args: &[&str]| {
            parse_args(args)
                .unwrap()
                .command
                .pdb_options()
                .unwrap()
                .solver
        };

        for command in ["solve", "filter-optimal", "opt-diff"] {
            assert_eq!(solver(&[command]), Pdb4x4Stm::Medium, "{command}");
            assert_eq!(
                solver(&[command, "--solver", "large"]),
                Pdb4x4Stm::Large,
                "{command}"
            );
        }
        assert_eq!(
            solver(&["optimize", "-l", "4", "--solver", "small"]),
            Pdb4x4Stm::Small
        );
        assert_eq!(
            solver(&["opt-diff", "-s", "4x4", "--solver", "huge"]),
            Pdb4x4Stm::Huge
        );

        assert!(parse_args(&["solve", "--solver", "tiny"]).is_err());
        assert!(parse_args(&["format", "--solver", "small"]).is_err());
    }

    #[test]
    fn cli_show_pdb_build() {
        for command in ["solve", "filter-optimal", "opt-diff"] {
            let args = parse_args(&[command, "--show-pdb-build"]).unwrap();
            assert!(
                args.command.pdb_options().unwrap().show_pdb_build,
                "{command}"
            );
            let args = parse_args(&[command]).unwrap();
            assert!(
                !args.command.pdb_options().unwrap().show_pdb_build,
                "{command}"
            );
        }
        let args = parse_args(&["optimize", "-l", "4", "--show-pdb-build"]).unwrap();
        assert!(args.command.pdb_options().unwrap().show_pdb_build);

        assert!(parse_args(&["format", "--show-pdb-build"]).is_err());
    }

    #[test]
    fn cli_apply_trace() {
        let trace = |args: &[&str]| match parse_args(&[&["apply", "-a", "R"], args].concat()) {
            Ok(Args {
                command: Command::Apply { trace, .. },
            }) => Ok(trace),
            Ok(args) => panic!("parsed as {args:?}"),
            Err(e) => Err(e.kind()),
        };

        assert_eq!(trace(&[]), Ok(None));
        assert_eq!(trace(&["--trace"]), Ok(Some(Metric::Stm)));
        assert_eq!(trace(&["--trace", "stm"]), Ok(Some(Metric::Stm)));
        assert_eq!(trace(&["--trace", "mtm"]), Ok(Some(Metric::Mtm)));
        assert_eq!(trace(&["--trace=mtm"]), Ok(Some(Metric::Mtm)));
        assert_eq!(trace(&["-t", "mtm"]), Ok(Some(Metric::Mtm)));
        assert_eq!(
            trace(&["--trace", "-s", "1 2/3 0"]),
            Ok(Some(Metric::Stm)),
            "a following option isn't taken as the metric"
        );
        assert_eq!(
            trace(&["--trace", "foo"]),
            Err(clap::error::ErrorKind::InvalidValue)
        );
        assert_eq!(
            trace(&["--metric", "mtm"]),
            Err(clap::error::ErrorKind::UnknownArgument)
        );
    }

    #[test]
    fn cli_generate_invertible() {
        assert!(parse_args(&["generate", "--invertible"]).is_ok());
        assert!(parse_args(&["generate", "--invertible", "--random-moves"]).is_err());
    }

    /// Applies `alg` to a large puzzle with the gap in the middle, so that any short algorithm can
    /// be applied regardless of how the gap moves.
    fn effect(alg: &Algorithm) -> Puzzle {
        let mut puzzle = Puzzle::new(Size::new(15, 15).unwrap());
        assert!(puzzle.try_apply_alg(&Algorithm::from_str("R7D7").unwrap()));
        assert!(puzzle.try_apply_alg(alg));
        puzzle
    }

    fn optimize(alg: &str, metric: Metric, length: u64) -> Algorithm {
        let original = Algorithm::from_str(alg).unwrap();
        let mut optimized = original.clone();
        runner()
            .optimize_alg(&mut optimized, metric, length)
            .unwrap();

        assert_eq!(
            effect(&optimized),
            effect(&original),
            "{alg} -> {optimized}"
        );
        assert!(optimized.len_metric(metric) <= original.len_metric(metric));

        optimized
    }

    #[test]
    fn optimize_simplifies_after_splicing() {
        // The optimal replacement for the last 8 moves starts with R, which cancels the leading
        // L.
        assert_eq!(optimize("LDRULDRUL", Metric::Stm, 8).to_string(), "DLU");
        assert_eq!(optimize("RDRULDRUL", Metric::Stm, 8).to_string(), "R2DLU");
    }

    #[test]
    fn optimize_rechecks_windows_before_the_splice() {
        // Splicing in an optimal replacement creates new improvable windows that start before the
        // splice point. Without rechecking those, this only gets down to 10 moves.
        let alg = optimize("URDLURDLLDRULDRULD", Metric::Stm, 8);
        assert_eq!(alg.to_string(), "RULD2L");

        let alg = optimize("DLURDLURDLRDLURDLURDLDRULDRULD", Metric::Stm, 8);
        assert_eq!(alg.len_stm(), 6, "{alg}");
    }

    #[test]
    fn optimize_output_is_a_fixed_point() {
        for alg in ["LDRULDRUL", "RDRULDRUL", "DRULDRULDRUL", "DRDRULDRULDRUL"] {
            for length in [2, 4, 6, 8] {
                let once = optimize(alg, Metric::Stm, length);
                let mut twice = once.clone();
                runner()
                    .optimize_alg(&mut twice, Metric::Stm, length)
                    .unwrap();
                assert_eq!(twice, once, "{alg} with length {length}");
            }
        }
    }

    #[test]
    fn optimize_trivial_cases() {
        assert_eq!(optimize("", Metric::Stm, 4).to_string(), "");
        assert_eq!(optimize("RU", Metric::Stm, 0).to_string(), "RU");
        assert_eq!(optimize("RU", Metric::Stm, 5).to_string(), "RU");
        assert_eq!(optimize("RLU", Metric::Stm, 5).to_string(), "U");
    }
}
