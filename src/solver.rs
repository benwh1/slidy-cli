pub mod projections;

use std::{
    cell::LazyCell,
    collections::HashMap,
    fs::File,
    io::{self, BufReader, BufWriter, Write as _},
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{
        label::label::{
            Checkerboard, Diagonals, Fringe, Label, RowGrids, Rows, SplitFringe, SplitSquareFringe,
            SquareFringe, Trivial,
        },
        puzzle::Puzzle,
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        generic_solver::GenericSolver,
        heuristic::{manhattan::ManhattanDistance, mtm::MtmHeuristic},
        projection::{pdb::Pdb as ProjectionPdb, solver::Solver as ProjectionSolver},
        solver::{Solver as SolverT, SolverError},
        statistics::PdbIterationStats,
        Solver4x4Mtm, Solver4x4Stm,
    },
};
use zstd::Encoder;

use crate::{
    cache_versions::SOLVER_VERSION,
    enums::{LabelType, Metric},
    solver::projections::{DiagonalsPruneTarget4x4, SplitFringePruneTarget4x4},
};

type BoxSolver = Box<dyn SolverT<Puzzle, Context = ()>>;
type BoxSolverInit = Box<dyn FnOnce() -> BoxSolver>;

type Remap = Box<dyn Fn(SolverKey) -> Option<SolverKey>>;

/// The directory PDBs are cached in, or `None` if the platform has no cache directory (e.g. no
/// home directory could be determined). The directory is versioned by [`SOLVER_VERSION`].
fn default_pdb_dir() -> Option<PathBuf> {
    let mut dir = ProjectDirs::from("", "", "slidy-cli")?
        .cache_dir()
        .to_path_buf();
    dir.push(format!("v{SOLVER_VERSION}"));
    dir.push("solver");
    dir.push("pdb");

    Some(dir)
}

/// The move set PDBs are built for. Only the standard moves exist for now, but it is part of the
/// file name so that other move sets can be added without renaming existing files.
const MOVE_SET: &str = "standard";

/// The file name of a cached PDB, `<width>x<height>-<metric>-<solved state>-<move set>.pdb.zst`,
/// e.g. `4x4-stm-fringe-standard.pdb.zst`.
///
/// `solved_state` is the label the solver solves to. For projection solvers that is the target,
/// even when a different prune target is used: prune targets are fixed per registration, and
/// changing one requires a `SOLVER_VERSION` bump, which moves the whole cache to a new directory.
fn pdb_file_name(size: Size, metric: Metric, solved_state: &str) -> String {
    format!(
        "{w}x{h}-{metric}-{solved_state}-{MOVE_SET}.pdb.zst",
        w = size.width(),
        h = size.height(),
        metric = metric.pdb_file_name(),
    )
}

/// Hands out PDB file paths while solvers are registered, and checks that no two different kinds
/// of PDB are given the same file.
///
/// File names don't say which solver a PDB belongs to, so e.g. a projection solver for `row-grids`
/// would get the same name as a small solver's PDB, despite the formats being different. Each one
/// would then reject the other's file and rebuild it on every use.
struct PdbPaths<'a> {
    dir: Option<&'a Path>,
    owners: HashMap<String, &'static str>,
}

impl<'a> PdbPaths<'a> {
    fn new(dir: Option<&'a Path>) -> Self {
        Self {
            dir,
            owners: HashMap::new(),
        }
    }

    /// The path of a PDB, or `None` if PDBs aren't being cached. `owner` identifies the PDB
    /// format: registrations that share a file must use the same `owner`.
    ///
    /// # Panics
    ///
    /// If the file has already been given to a different `owner`.
    fn path(
        &mut self,
        size: Size,
        metric: Metric,
        solved_state: &str,
        owner: &'static str,
    ) -> Option<PathBuf> {
        let name = pdb_file_name(size, metric, solved_state);

        let previous = *self.owners.entry(name.clone()).or_insert(owner);
        assert_eq!(
            previous, owner,
            "PDB file {name} would be shared by {previous} and {owner}"
        );

        Some(self.dir?.join(name))
    }
}

/// The number of entries in the projection PDB that `slidy` builds for `prune_target` on a puzzle
/// of size `size`.
///
/// This mirrors `slidy`'s own computation. The PDB has one entry per arrangement of the gap and the
/// labelled pieces, which is the multinomial coefficient of the label counts. The projection
/// solvers only index the PDB with encodings of projected puzzles, which are a bijection onto
/// `0..len`, so a PDB of exactly this length can't be indexed out of bounds.
fn projection_pdb_len<L>(prune_target: &L, size: Size) -> Option<usize>
where
    L: Label,
{
    let mut counts = HashMap::<u64, u64>::new();
    for i in 0..size.num_pieces() {
        let pos = (i % size.width(), i / size.width());
        *counts
            .entry(prune_target.position_label(size, pos))
            .or_default() += 1;
    }

    // The gap is its own label, with a count of 1.
    let mut remaining = size.area();
    let mut len = 1u128;
    for count in counts.into_values().chain([1]) {
        len = len.checked_mul(binomial(remaining, count)?)?;
        remaining -= count;
    }

    usize::try_from(len).ok()
}

fn binomial(n: u64, k: u64) -> Option<u128> {
    // After step `i`, `r == C(n, i + 1)`, so every division is exact.
    let mut r = 1u128;
    for i in 0..k {
        r = r.checked_mul(u128::from(n - i))? / u128::from(i + 1);
    }

    Some(r)
}

/// Reads and decompresses a cached PDB. Returns `None` if there is no cache, the file doesn't
/// exist, or it can't be decompressed.
fn read_compressed_pdb(pdb_file_path: Option<&Path>) -> Option<Box<[u8]>> {
    let file = File::open(pdb_file_path?).ok()?;
    let bytes = zstd::decode_all(BufReader::new(file)).ok()?;

    Some(bytes.into_boxed_slice())
}

/// Compresses `bytes` and writes them to `pdb_file_path` atomically.
///
/// The data is written to a temporary file next to the target, synced, and then renamed over the
/// target. Readers therefore only ever see either no file or a complete one, even if this process
/// is killed mid-write or another process is writing the same PDB at the same time.
fn write_compressed_pdb(pdb_file_path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = pdb_file_path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let tmp_path = tmp_file_path(pdb_file_path);
    let result = write_compressed_file(&tmp_path, bytes)
        .and_then(|()| std::fs::rename(&tmp_path, pdb_file_path));

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }

    result
}

/// A temporary file path in the same directory as `path` (so that renaming it over `path` is
/// atomic), unique to this process.
fn tmp_file_path(path: &Path) -> PathBuf {
    let mut file_name = path.file_name().unwrap_or_default().to_owned();
    file_name.push(format!(".tmp.{}", std::process::id()));
    path.with_file_name(file_name)
}

fn write_compressed_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = Encoder::new(writer, 0)?;
    encoder.include_checksum(true)?;
    encoder.include_contentsize(true)?;
    encoder.write_all(bytes)?;

    let file = encoder
        .finish()?
        .into_inner()
        .map_err(io::IntoInnerError::into_error)?;
    file.sync_all()
}

/// Saves a freshly built PDB to the cache. Failing to save is not fatal: the PDB is still usable
/// in memory, it just has to be rebuilt next time.
fn save_pdb(pdb_file_path: Option<&Path>, bytes: &[u8]) {
    let Some(pdb_file_path) = pdb_file_path else {
        eprintln!("warning: no cache directory available, so the pattern database was not saved");
        return;
    };

    if let Err(e) = write_compressed_pdb(pdb_file_path, bytes) {
        eprintln!(
            "warning: failed to save pattern database to {}: {e}",
            pdb_file_path.display()
        );
    }
}

fn pdb_config() -> PdbConfig {
    PdbConfig {
        end_of_iter_callback: Some(Box::new(|s: PdbIterationStats| {
            eprintln!("depth {} new {} total {}", s.depth, s.new, s.total);
        })),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SolverKey {
    size: Size,
    label: LabelType,
    metric: Metric,
}

pub struct Solver {
    solvers: HashMap<SolverKey, LazyCell<BoxSolver, BoxSolverInit>>,
    remaps: Vec<Remap>,
}

impl Solver {
    /// Creates a solver that caches PDBs in the user's cache directory.
    pub fn new() -> Self {
        Self::with_pdb_dir(default_pdb_dir().as_deref())
    }

    /// Creates a solver that caches PDBs in `pdb_dir`, or doesn't cache them at all if `pdb_dir`
    /// is `None`. The directory is only created when a PDB is first saved.
    pub fn with_pdb_dir(pdb_dir: Option<&Path>) -> Self {
        let mut this = Self {
            solvers: HashMap::new(),
            remaps: Vec::new(),
        };

        let mut pdb_paths = PdbPaths::new(pdb_dir);
        let row_grids = LabelType::RowGrids.pdb_file_name();

        macro_rules! small {
            ($pdb_file_path:expr, $pdb_ty:ty, $solver_ty:ty) => {{
                let pdb_file_path = $pdb_file_path.clone();

                move || {
                    type PdbTy = $pdb_ty;
                    type SolverTy = $solver_ty;

                    let pdb = read_compressed_pdb(pdb_file_path.as_deref())
                        .and_then(|bytes| {
                            // SAFETY: this computes a checksum to verify correctness, which is
                            // good enough here.
                            unsafe { PdbTy::try_from_bytes(bytes) }
                        })
                        .unwrap_or_else(|| {
                            let pdb = PdbTy::new(&pdb_config());

                            let bytes = pdb.as_ref();
                            save_pdb(pdb_file_path.as_deref(), bytes);

                            pdb
                        });

                    SolverTy::with_pdb(pdb)
                }
            }};
        }

        macro_rules! register_small_solver {
            ($max:literal, $min:literal) => {{
                use slidy::solver::small::{pdb::Pdb, solver::Solver};

                type PdbStm = Pdb<$max, $min, { $max * $min }, Stm>;
                type PdbMtm = Pdb<$max, $min, { $max * $min }, Mtm>;
                type SolverStm = Solver<$max, $min, { $max * $min }, Stm>;
                type SolverMtm = Solver<$max, $min, { $max * $min }, Mtm>;

                let max = $max;
                let min = $min;

                let size = Size::new(max, min).unwrap();

                let file_name_stm = pdb_paths.path(size, Metric::Stm, row_grids, "small solver");
                let file_name_mtm = pdb_paths.path(size, Metric::Mtm, row_grids, "small solver");

                this.register(
                    size,
                    LabelType::RowGrids,
                    Metric::Stm,
                    small!(file_name_stm, PdbStm, SolverStm),
                );
                this.register(
                    size,
                    LabelType::RowGrids,
                    Metric::Mtm,
                    small!(file_name_mtm, PdbMtm, SolverMtm),
                );

                // The small solvers also solve the transposed size, so share one solver (and one
                // copy of its PDB) between both orientations. The remapped key has the wrong size
                // for the puzzle, which is fine because the solver is still given the original
                // puzzle.
                if max != min {
                    let transposed = size.transpose();

                    this.register_remap(move |key| {
                        (key.size == transposed && key.label == LabelType::RowGrids)
                            .then_some(SolverKey { size, ..key })
                    });
                }
            }};
        }

        // Small puzzles

        register_small_solver!(2, 2);
        register_small_solver!(3, 2);
        register_small_solver!(3, 3);
        register_small_solver!(4, 2);
        register_small_solver!(4, 3);
        register_small_solver!(5, 2);
        register_small_solver!(6, 2);

        // 4x4 STM and MTM

        this.register(
            Size::new(4, 4).unwrap(),
            LabelType::RowGrids,
            Metric::Stm,
            Solver4x4Stm::default,
        );

        let size = Size::new(4, 4).unwrap();
        let pdb_file_path_4x4_mtm = pdb_paths.path(size, Metric::Mtm, row_grids, "4x4 MTM solver");
        this.register(size, LabelType::RowGrids, Metric::Mtm, move || {
            read_compressed_pdb(pdb_file_path_4x4_mtm.as_deref())
                .and_then(|bytes| {
                    // SAFETY: this computes a checksum to verify correctness, which is good
                    // enough here.
                    unsafe { Solver4x4Mtm::try_with_pdb_bytes(bytes) }
                })
                .unwrap_or_else(|| {
                    let solver = Solver4x4Mtm::new(&pdb_config());

                    let bytes = solver.pdb().as_ref();
                    save_pdb(pdb_file_path_4x4_mtm.as_deref(), bytes);

                    solver
                })
        });

        // Labels

        macro_rules! register_projection_solver {
            ($w:literal, $h:literal, $target:tt, $prune_target:tt, $metric:tt) => {{
                let w = $w;
                let h = $h;
                let target = LabelType::$target;
                let metric = Metric::$metric;

                let size = Size::new(w, h).unwrap();

                let pdb_file_path = pdb_paths.path(
                    size,
                    metric,
                    target.pdb_file_name(),
                    std::any::type_name::<$prune_target>(),
                );

                let solver = move || {
                    read_compressed_pdb(pdb_file_path.as_deref())
                        .and_then(|bytes| {
                            if bytes.len() != projection_pdb_len(&$prune_target, size)? {
                                return None;
                            }

                            // SAFETY: The length check above means the solver can't index the PDB
                            // out of bounds (see `projection_pdb_len`). The cache directory is
                            // versioned by `SOLVER_VERSION`, which is bumped whenever a PDB encoding
                            // or prune target changes, so the contents were built the same way, and
                            // the zstd checksum guards against corruption. A stale or tampered file
                            // of the same length could only give wrong (but memory-safe) heuristic
                            // values.
                            let pdb = unsafe { ProjectionPdb::from_bytes_unchecked(bytes) };

                            ProjectionSolver::builder()
                                .size(size)
                                .target($target)
                                .prune_target($prune_target)
                                .metric($metric)
                                .pdb(pdb)
                                .build()
                                .ok()
                        })
                        .unwrap_or_else(|| {
                            let solver = ProjectionSolver::builder()
                                .size(size)
                                .target($target)
                                .prune_target($prune_target)
                                .metric($metric)
                                .pdb_config(pdb_config())
                                .build()
                                .unwrap();

                            let bytes = solver.pdb().as_ref();
                            save_pdb(pdb_file_path.as_deref(), bytes);

                            solver
                        })
                };

                this.register(size, target, metric, solver);
            }};
            ($w:literal, $h:literal, $target:tt, $prune_target:tt) => {
                register_projection_solver!($w, $h, $target, $prune_target, Stm);
                register_projection_solver!($w, $h, $target, $prune_target, Mtm);
            };
            ($w:literal, $h:literal, $target:tt) => {
                register_projection_solver!($w, $h, $target, $target);
            };
        }

        // Rows

        register_projection_solver!(2, 2, Rows);
        register_projection_solver!(2, 3, Rows);
        register_projection_solver!(2, 4, Rows);
        register_projection_solver!(2, 5, Rows);
        register_projection_solver!(2, 6, Rows);

        register_projection_solver!(3, 2, Rows);
        register_projection_solver!(3, 3, Rows);
        register_projection_solver!(3, 4, Rows);
        register_projection_solver!(3, 5, Rows);

        register_projection_solver!(4, 2, Rows);
        register_projection_solver!(4, 3, Rows);
        register_projection_solver!(4, 4, Rows);

        register_projection_solver!(5, 2, Rows);
        register_projection_solver!(5, 3, Rows);

        register_projection_solver!(6, 2, Rows);
        register_projection_solver!(7, 2, Rows);
        register_projection_solver!(8, 2, Rows);
        register_projection_solver!(9, 2, Rows);
        register_projection_solver!(10, 2, Rows);
        register_projection_solver!(11, 2, Rows);
        register_projection_solver!(12, 2, Rows);

        // Fringe
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Fringe);
        register_projection_solver!(2, 3, Fringe);
        register_projection_solver!(2, 4, Fringe);
        register_projection_solver!(2, 5, Fringe);
        register_projection_solver!(2, 6, Fringe);
        register_projection_solver!(2, 7, Fringe);
        register_projection_solver!(2, 8, Fringe);
        register_projection_solver!(2, 9, Fringe);
        register_projection_solver!(2, 10, Fringe);
        register_projection_solver!(2, 11, Fringe);
        register_projection_solver!(2, 12, Fringe);
        register_projection_solver!(2, 13, Fringe);
        register_projection_solver!(2, 14, Fringe);

        register_projection_solver!(3, 2, Fringe);
        register_projection_solver!(3, 3, Fringe);
        register_projection_solver!(3, 4, Fringe);
        register_projection_solver!(3, 5, Fringe);
        register_projection_solver!(3, 6, Fringe);

        register_projection_solver!(4, 2, Fringe);
        register_projection_solver!(4, 3, Fringe);
        register_projection_solver!(4, 4, Fringe);

        register_projection_solver!(5, 2, Fringe);
        register_projection_solver!(6, 2, Fringe);
        register_projection_solver!(7, 2, Fringe);
        register_projection_solver!(8, 2, Fringe);
        register_projection_solver!(9, 2, Fringe);
        register_projection_solver!(10, 2, Fringe);
        register_projection_solver!(11, 2, Fringe);
        register_projection_solver!(12, 2, Fringe);
        register_projection_solver!(13, 2, Fringe);
        register_projection_solver!(14, 2, Fringe);

        // Square fringe
        // NxN is equivalent to fringe

        register_projection_solver!(2, 3, SquareFringe);
        register_projection_solver!(2, 4, SquareFringe);
        register_projection_solver!(2, 5, SquareFringe);
        register_projection_solver!(2, 6, SquareFringe);
        register_projection_solver!(2, 7, SquareFringe);

        register_projection_solver!(3, 2, SquareFringe);
        register_projection_solver!(3, 4, SquareFringe);

        register_projection_solver!(4, 2, SquareFringe);
        register_projection_solver!(4, 3, SquareFringe);

        register_projection_solver!(5, 2, SquareFringe);
        register_projection_solver!(5, 3, SquareFringe);

        register_projection_solver!(6, 2, SquareFringe);
        register_projection_solver!(7, 2, SquareFringe);

        // Split fringe

        register_projection_solver!(2, 2, SplitFringe);
        register_projection_solver!(2, 3, SplitFringe);
        register_projection_solver!(2, 4, SplitFringe);
        register_projection_solver!(2, 5, SplitFringe);
        register_projection_solver!(2, 6, SplitFringe);
        register_projection_solver!(2, 7, SplitFringe);
        register_projection_solver!(2, 8, SplitFringe);
        register_projection_solver!(2, 9, SplitFringe);
        register_projection_solver!(2, 10, SplitFringe);

        register_projection_solver!(3, 2, SplitFringe);
        register_projection_solver!(3, 3, SplitFringe);
        register_projection_solver!(3, 4, SplitFringe);
        register_projection_solver!(3, 5, SplitFringe);

        register_projection_solver!(4, 2, SplitFringe);
        register_projection_solver!(4, 3, SplitFringe);
        register_projection_solver!(4, 4, SplitFringe, SplitFringePruneTarget4x4);

        register_projection_solver!(5, 2, SplitFringe);
        register_projection_solver!(6, 2, SplitFringe);
        register_projection_solver!(7, 2, SplitFringe);
        register_projection_solver!(8, 2, SplitFringe);
        register_projection_solver!(9, 2, SplitFringe);
        register_projection_solver!(10, 2, SplitFringe);

        // Split square fringe
        // 2xN is equivalent to rows
        // NxN is equivalent to split fringe

        register_projection_solver!(3, 2, SplitSquareFringe);
        register_projection_solver!(3, 4, SplitSquareFringe);

        register_projection_solver!(4, 2, SplitSquareFringe);
        register_projection_solver!(4, 3, SplitSquareFringe);

        register_projection_solver!(5, 2, SplitSquareFringe);
        register_projection_solver!(6, 2, SplitSquareFringe);
        register_projection_solver!(7, 2, SplitSquareFringe);

        // Diagonals
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Diagonals);
        register_projection_solver!(2, 3, Diagonals);
        register_projection_solver!(2, 4, Diagonals);
        register_projection_solver!(2, 5, Diagonals);
        register_projection_solver!(2, 6, Diagonals);

        register_projection_solver!(3, 2, Diagonals);
        register_projection_solver!(3, 3, Diagonals);
        register_projection_solver!(3, 4, Diagonals);

        register_projection_solver!(4, 2, Diagonals);
        register_projection_solver!(4, 3, Diagonals);
        register_projection_solver!(4, 4, Diagonals, DiagonalsPruneTarget4x4);

        register_projection_solver!(5, 2, Diagonals);
        register_projection_solver!(6, 2, Diagonals);
        register_projection_solver!(7, 2, Diagonals);

        // Checkerboard
        // TODO: make small x big transpose to big x small

        register_projection_solver!(2, 2, Checkerboard);
        register_projection_solver!(2, 3, Checkerboard);
        register_projection_solver!(2, 4, Checkerboard);
        register_projection_solver!(2, 5, Checkerboard);
        register_projection_solver!(2, 6, Checkerboard);
        register_projection_solver!(2, 7, Checkerboard);
        register_projection_solver!(2, 8, Checkerboard);
        register_projection_solver!(2, 9, Checkerboard);
        register_projection_solver!(2, 10, Checkerboard);
        register_projection_solver!(2, 11, Checkerboard);
        register_projection_solver!(2, 12, Checkerboard);
        register_projection_solver!(2, 13, Checkerboard);
        register_projection_solver!(2, 14, Checkerboard);

        register_projection_solver!(3, 2, Checkerboard);
        register_projection_solver!(3, 3, Checkerboard);
        register_projection_solver!(3, 4, Checkerboard);
        register_projection_solver!(3, 5, Checkerboard);
        register_projection_solver!(3, 6, Checkerboard);
        register_projection_solver!(3, 7, Checkerboard);
        register_projection_solver!(3, 8, Checkerboard);
        register_projection_solver!(3, 9, Checkerboard);

        register_projection_solver!(4, 2, Checkerboard);
        register_projection_solver!(4, 3, Checkerboard);
        register_projection_solver!(4, 4, Checkerboard);
        register_projection_solver!(4, 5, Checkerboard);
        register_projection_solver!(4, 6, Checkerboard);
        register_projection_solver!(4, 7, Checkerboard);

        register_projection_solver!(5, 2, Checkerboard);
        register_projection_solver!(5, 3, Checkerboard);
        register_projection_solver!(5, 4, Checkerboard);
        register_projection_solver!(5, 5, Checkerboard);

        register_projection_solver!(6, 2, Checkerboard);
        register_projection_solver!(6, 3, Checkerboard);
        register_projection_solver!(6, 4, Checkerboard);

        register_projection_solver!(7, 2, Checkerboard);
        register_projection_solver!(7, 3, Checkerboard);
        register_projection_solver!(7, 4, Checkerboard);

        register_projection_solver!(8, 2, Checkerboard);
        register_projection_solver!(8, 3, Checkerboard);

        register_projection_solver!(9, 2, Checkerboard);
        register_projection_solver!(9, 3, Checkerboard);

        register_projection_solver!(10, 2, Checkerboard);
        register_projection_solver!(11, 2, Checkerboard);
        register_projection_solver!(12, 2, Checkerboard);
        register_projection_solver!(13, 2, Checkerboard);
        register_projection_solver!(14, 2, Checkerboard);

        // Remaps

        // Map square fringe to normal fringe on square puzzles
        this.register_remap(|key| {
            if !key.size.is_square() {
                return None;
            }

            let new_label = match key.label {
                LabelType::SquareFringe => LabelType::Fringe,
                LabelType::SplitSquareFringe => LabelType::SplitFringe,
                LabelType::Trivial
                | LabelType::RowGrids
                | LabelType::Rows
                | LabelType::Fringe
                | LabelType::SplitFringe
                | LabelType::Diagonals
                | LabelType::Checkerboard => return None,
            };

            Some(SolverKey {
                label: new_label,
                ..key
            })
        });

        // Map split square fringe 2xN to rows
        this.register_remap(|key| {
            (key.size.width() == 2 && key.label == LabelType::SplitSquareFringe).then_some(
                SolverKey {
                    label: LabelType::Rows,
                    ..key
                },
            )
        });

        this
    }

    fn register<S, F>(&mut self, size: Size, label: LabelType, metric: Metric, solver: F)
    where
        S: SolverT<Puzzle, Context = ()> + 'static,
        F: FnOnce() -> S + 'static,
    {
        self.solvers.insert(
            SolverKey {
                size,
                label,
                metric,
            },
            LazyCell::new(Box::new(move || Box::new(solver()))),
        );
    }

    fn register_remap<F>(&mut self, remap: F)
    where
        F: Fn(SolverKey) -> Option<SolverKey> + 'static,
    {
        self.remaps.push(Box::new(remap));
    }

    fn remap_key(&self, mut key: SolverKey) -> SolverKey {
        while let Some(new_key) = self.remaps.iter().find_map(|remap| remap(key)) {
            key = new_key;
        }

        key
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SolverContext {
    pub metric: Metric,
    pub label: LabelType,
}

impl SolverT<Puzzle> for Solver {
    type Context = SolverContext;

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &Puzzle,
        config: SolverConfig,
        context: &Self::Context,
    ) -> Result<(), SolverError> {
        let key = self.remap_key(SolverKey {
            size: puzzle.size(),
            label: context.label,
            metric: context.metric,
        });

        match self.solvers.get_mut(&key) {
            Some(s) => s.solve_with_config(puzzle, config),
            None => fallback_solver(SolverContext {
                metric: key.metric,
                label: key.label,
            })
            .solve_with_config(puzzle, config),
        }
    }
}

fn fallback_solver(context: SolverContext) -> Box<dyn SolverT<Puzzle, Context = ()>> {
    let SolverContext { metric, label } = context;

    match (metric, label) {
        (Metric::Stm, LabelType::Trivial) => {
            Box::new(GenericSolver::new(ManhattanDistance(Trivial), Trivial))
        }
        (Metric::Stm, LabelType::RowGrids) => {
            Box::new(GenericSolver::new(ManhattanDistance(RowGrids), RowGrids))
        }
        (Metric::Stm, LabelType::Rows) => {
            Box::new(GenericSolver::new(ManhattanDistance(Rows), Rows))
        }
        (Metric::Stm, LabelType::Fringe) => {
            Box::new(GenericSolver::new(ManhattanDistance(Fringe), Fringe))
        }
        (Metric::Stm, LabelType::SquareFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SquareFringe),
            SquareFringe,
        )),
        (Metric::Stm, LabelType::SplitFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SplitFringe),
            SplitFringe,
        )),
        (Metric::Stm, LabelType::SplitSquareFringe) => Box::new(GenericSolver::new(
            ManhattanDistance(SplitSquareFringe),
            SplitSquareFringe,
        )),
        (Metric::Stm, LabelType::Diagonals) => {
            Box::new(GenericSolver::new(ManhattanDistance(Diagonals), Diagonals))
        }
        (Metric::Stm, LabelType::Checkerboard) => Box::new(GenericSolver::new(
            ManhattanDistance(Checkerboard),
            Checkerboard,
        )),
        (Metric::Mtm, LabelType::Trivial) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Trivial)),
            Trivial,
        )),
        (Metric::Mtm, LabelType::RowGrids) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(RowGrids)),
            RowGrids,
        )),
        (Metric::Mtm, LabelType::Rows) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Rows)),
            Rows,
        )),
        (Metric::Mtm, LabelType::Fringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Fringe)),
            Fringe,
        )),
        (Metric::Mtm, LabelType::SquareFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SquareFringe)),
            SquareFringe,
        )),
        (Metric::Mtm, LabelType::SplitFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SplitFringe)),
            SplitFringe,
        )),
        (Metric::Mtm, LabelType::SplitSquareFringe) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(SplitSquareFringe)),
            SplitSquareFringe,
        )),
        (Metric::Mtm, LabelType::Diagonals) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Diagonals)),
            Diagonals,
        )),
        (Metric::Mtm, LabelType::Checkerboard) => Box::new(GenericSolver::new(
            MtmHeuristic(ManhattanDistance(Checkerboard)),
            Checkerboard,
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn projection_pdb_len_matches_built_pdb() {
        macro_rules! check {
            ($w:literal, $h:literal, $target:expr, $metric:expr) => {{
                let size = Size::new($w, $h).unwrap();
                let solver = ProjectionSolver::<Puzzle, _, _, _>::builder()
                    .size(size)
                    .target($target)
                    .prune_target($target)
                    .metric($metric)
                    .build()
                    .unwrap();

                assert_eq!(
                    Some(solver.pdb().as_ref().len()),
                    projection_pdb_len(&$target, size),
                    "{}x{} {}",
                    $w,
                    $h,
                    stringify!($target),
                );
            }};
        }

        check!(2, 2, Rows, Stm);
        check!(3, 2, Rows, Mtm);
        check!(2, 3, Fringe, Stm);
        check!(3, 3, Fringe, Stm);
        check!(3, 3, Diagonals, Mtm);
        check!(4, 2, SplitFringe, Stm);
        check!(2, 4, SquareFringe, Stm);
        check!(3, 2, SplitSquareFringe, Stm);
        check!(3, 3, Checkerboard, Stm);
        check!(4, 4, Checkerboard, Stm);
    }

    #[test]
    fn projection_pdb_len_custom_prune_targets() {
        let size = Size::new(4, 4).unwrap();

        // Label counts (7, 3, 2, 2, 1) plus the gap: 16! / (7! 3! 2! 2! 1! 1!).
        assert_eq!(
            projection_pdb_len(&SplitFringePruneTarget4x4, size),
            Some(172_972_800)
        );
        // Label counts (1, 2, 7, 3, 2) plus the gap: the same multinomial.
        assert_eq!(
            projection_pdb_len(&DiagonalsPruneTarget4x4, size),
            Some(172_972_800)
        );
    }

    #[test]
    fn pdb_file_names() {
        let name = |w, h, metric, solved_state| {
            pdb_file_name(Size::new(w, h).unwrap(), metric, solved_state)
        };

        assert_eq!(
            name(4, 3, Metric::Stm, LabelType::RowGrids.pdb_file_name()),
            "4x3-stm-row-grids-standard.pdb.zst"
        );
        assert_eq!(
            name(
                4,
                4,
                Metric::Stm,
                LabelType::SplitSquareFringe.pdb_file_name()
            ),
            "4x4-stm-split-square-fringe-standard.pdb.zst"
        );
    }

    #[test]
    fn pdb_paths() {
        let dir = Path::new("pdb");
        let size = Size::new(3, 3).unwrap();

        let mut paths = PdbPaths::new(Some(dir));
        assert_eq!(
            paths.path(size, Metric::Stm, "rows", "a"),
            Some(dir.join("3x3-stm-rows-standard.pdb.zst"))
        );
        // The same owner may share a file.
        assert!(paths.path(size, Metric::Stm, "rows", "a").is_some());

        assert_eq!(
            PdbPaths::new(None).path(size, Metric::Stm, "rows", "a"),
            None
        );
    }

    #[test]
    #[should_panic(expected = "would be shared by a and b")]
    fn pdb_paths_reject_sharing_between_owners() {
        let size = Size::new(3, 3).unwrap();
        let mut paths = PdbPaths::new(None);
        let _ = paths.path(size, Metric::Stm, "rows", "a");
        let _ = paths.path(size, Metric::Stm, "rows", "b");
    }

    #[test]
    fn registered_pdb_files_are_not_shared_between_formats() {
        // Registering every solver runs the `PdbPaths` checks without building any PDBs.
        let _ = Solver::with_pdb_dir(None);
    }

    #[test]
    fn pdb_write_replaces_atomically() {
        let dir =
            std::env::temp_dir().join(format!("slidy-cli-test-pdb-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("test.pdb.zst");
        let files = || std::fs::read_dir(path.parent().unwrap()).unwrap().count();

        write_compressed_pdb(&path, b"old contents").unwrap();
        write_compressed_pdb(&path, b"new").unwrap();
        assert_eq!(
            read_compressed_pdb(Some(&path)).as_deref(),
            Some(&b"new"[..])
        );
        assert_eq!(files(), 1, "temporary file left behind");

        // If the rename fails, the temporary file is cleaned up.
        let blocked = dir.join("nested").join("blocked");
        std::fs::create_dir_all(blocked.join("non-empty")).unwrap();
        assert!(write_compressed_pdb(&blocked, b"data").is_err());
        assert_eq!(files(), 2, "temporary file left behind");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn transposed_small_sizes_share_a_solver() {
        let mut solver = Solver::with_pdb_dir(None);
        let key = |w, h, metric| SolverKey {
            size: Size::new(w, h).unwrap(),
            label: LabelType::RowGrids,
            metric,
        };

        for metric in [Metric::Stm, Metric::Mtm] {
            assert_eq!(solver.remap_key(key(2, 3, metric)), key(3, 2, metric));
            assert_eq!(solver.remap_key(key(3, 2, metric)), key(3, 2, metric));
            assert!(!solver.solvers.contains_key(&key(2, 3, metric)));

            for state in ["0 3/2 1/5 4", "4 1 2/5 0 3"] {
                let mut puzzle = Puzzle::from_str(state).unwrap();
                let context = SolverContext {
                    metric,
                    label: LabelType::RowGrids,
                };
                let solution = solver.solve_with_context(&puzzle, &context).unwrap();

                assert!(puzzle.try_apply_alg(&solution), "{state}: {solution}");
                assert!(puzzle.is_solved(), "{state}: {solution}");
            }
        }
    }

    #[test]
    fn binomial_values() {
        assert_eq!(binomial(5, 0), Some(1));
        assert_eq!(binomial(5, 2), Some(10));
        assert_eq!(binomial(16, 8), Some(12870));
        assert_eq!(binomial(30, 15), Some(155_117_520));
    }
}
