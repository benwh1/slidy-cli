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
/// - `slidy` changes how a PDB is built, encoded or indexed (see [`SLIDY_VERSION`]),
/// - a solver registration changes what a PDB contains without changing its file name,
/// - the file naming, compression or layout of the cache changes.
///
/// Projection PDBs are loaded without a hash check, so reusing a stale one gives wrong (but
/// memory-safe, thanks to the length check) results. When in doubt, bump.
pub const SOLVER_VERSION: &str = "0.6";

/// The `slidy` version that [`SOLVER_VERSION`] was last checked against.
///
/// A test compares this with `Cargo.lock`. When `slidy` is upgraded, the test fails until someone
/// checks whether the new version changes any PDB format, bumps [`SOLVER_VERSION`] if it does, and
/// then updates this constant. A `slidy` upgrade that doesn't affect PDBs therefore keeps the
/// existing cache.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "only read by the Cargo.lock check in the tests")
)]
pub const SLIDY_VERSION: &str = "0.7.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slidy_version_matches_cargo_lock() {
        let lock = include_str!("../Cargo.lock");
        let slidy_version = lock
            .split("[[package]]")
            .find(|package| package.contains("\nname = \"slidy\"\n"))
            .and_then(|package| {
                package
                    .lines()
                    .find_map(|line| line.strip_prefix("version = \""))
                    .and_then(|v| v.strip_suffix('"'))
            })
            .expect("slidy is in Cargo.lock");

        assert_eq!(
            slidy_version, SLIDY_VERSION,
            "slidy was upgraded. Check whether this changes any PDB format: if it does, bump \
            SOLVER_VERSION to the upcoming release version. Then set SLIDY_VERSION to \
            {slidy_version}."
        );
    }
}
