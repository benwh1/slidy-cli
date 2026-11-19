//! Versions of the on-disk cache formats.
//!
//! Each cache lives in a directory named after its version. Leaving a version unchanged across
//! releases keeps reusing the existing cache files, and bumping it makes everything in that cache
//! get rebuilt (the old directory is simply no longer read).

/// The slidy-cli version in which the solver cache (pattern databases) last changed.
///
/// Bump this to the upcoming release's version whenever a cached PDB could differ from what the
/// current code would build or expect, for example when:
///
/// - `slidy` changes how a PDB is built, encoded or indexed (it is pinned in `Cargo.toml`, so that
///   upgrading it is a reminder to check),
/// - a solver registration changes what a PDB contains without changing its file name, e.g. a
///   projection solver's prune target (file names only include the target),
/// - the file naming, compression or layout of the cache changes.
///
/// Projection PDBs are loaded without a hash check, so reusing a stale one is memory-safe (thanks
/// to the length check) but can give wrong solutions, panic, or hang. When in doubt, bump.
pub const SOLVER_VERSION: &str = "0.6";
