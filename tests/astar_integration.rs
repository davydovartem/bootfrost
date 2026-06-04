//! Integration tests for the A* script at `tests/scripts/astar.rhai`.
//!
//! These tests load a real `.rhai` file from disk, pass 2D grids and
//! coordinates in, and check that `a_star_step` returns the expected
//! first step of the shortest path. The fixture lives at
//! `tests/scripts/astar.rhai`, resolved via `CARGO_MANIFEST_DIR`.
//!
//! The `grid(...)` helper builds a Rhai `Array of Array of i64` from
//! a Rust `&[&[i64]]`, matching the way Bootfrost's `Term::List`
//! representation will be marshalled into Rhai at integration time.

use bootfrost::strategies::rhai_runtime::RhaiRuntime;
use rhai::{Array, Dynamic};

/// Absolute path to the bundled A* verification script.
fn astar_script_path() -> String {
    format!(
        "{}/tests/scripts/astar.rhai",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Open the bundled A* script. Panics with a clear message if the
/// fixture is missing, so a stale file is reported loudly.
fn load_astar_script() -> RhaiRuntime {
    let path = astar_script_path();
    RhaiRuntime::from_file(&path)
        .unwrap_or_else(|e| panic!("failed to load '{}': {}", path, e))
}

/// Build a 2D Rhai array from a slice-of-slices. Each row of `rows`
/// becomes a Rhai `Array` of `i64`, and the rows themselves are
/// collected into the outer `Array`. Each row is wrapped in a
/// `Dynamic` so the outer iterator yields `Dynamic` items and the
/// final `collect()` produces an `Array` of rows.
fn grid(rows: &[&[i64]]) -> Array {
    rows.iter()
        .map(|row| {
            let inner: Array = row.iter().map(|&v| Dynamic::from(v)).collect();
            Dynamic::from(inner)
        })
        .collect()
}

/// Pull the `[x, y]` integers out of a Rhai array.
fn xy(arr: Array) -> (i64, i64) {
    assert_eq!(arr.len(), 2, "a_star_step must return a 2-element array");
    let x = arr[0]
        .as_int()
        .expect("first element of result must be an integer");
    let y = arr[1]
        .as_int()
        .expect("second element of result must be an integer");
    (x, y)
}

// ----------------------------------------------------------------------
// Trivial cases
// ----------------------------------------------------------------------

#[test]
fn a_star_step_returns_start_when_already_at_goal() {
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1, 1], &[1, 1, 1], &[1, 1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (1i64, 1i64, 1i64, 1i64, cells))
        .unwrap();
    assert_eq!(xy(result), (1, 1));
}

#[test]
fn a_star_step_returns_minus_one_on_empty_grid() {
    let rt = load_astar_script();
    let cells: Array = vec![];
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 1i64, 1i64, cells))
        .unwrap();
    assert_eq!(xy(result), (-1, -1));
}

// ----------------------------------------------------------------------
// Forced paths (no tie-breaking ambiguity)
// ----------------------------------------------------------------------

#[test]
fn a_star_step_finds_path_around_blocked_middle_row() {
    // Grid (1 = free, 0 = blocked):
    //   1 1 1
    //   1 0 1
    //   1 1 1
    // From (0,0) the only way to (2,2) is to walk around the blocked
    // centre cell. With the fixed neighbour order (right, left, down,
    // up) the first step is forced to (1, 0).
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1, 1], &[1, 0, 1], &[1, 1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 2i64, 2i64, cells))
        .unwrap();
    assert_eq!(xy(result), (1, 0));
}

#[test]
fn a_star_step_finds_path_through_corridor() {
    // Grid:
    //   1 1 1 1 1
    //   0 0 0 0 1
    //   1 1 1 1 1
    // From (0,0) the only path to (4,2) is along the top row, then
    // down the right column. First step is forced to (1, 0).
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1, 1, 1, 1], &[0, 0, 0, 0, 1], &[1, 1, 1, 1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 4i64, 2i64, cells))
        .unwrap();
    assert_eq!(xy(result), (1, 0));
}

#[test]
fn a_star_step_finds_path_through_vertical_corridor() {
    // Grid:
    //   1 0 1
    //   1 0 1
    //   1 0 1
    //   1 1 1
    // From (0,0) to (2,3) the only path is straight down to (0,3),
    // then right to (1,3), then right to (2,3). First step: (0, 1).
    let rt = load_astar_script();
    let cells = grid(&[&[1, 0, 1], &[1, 0, 1], &[1, 0, 1], &[1, 1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 2i64, 3i64, cells))
        .unwrap();
    assert_eq!(xy(result), (0, 1));
}

// ----------------------------------------------------------------------
// Free-grid behaviour: tie-breaking makes the answer deterministic
// because neighbour directions are explored in a fixed order
// (right, left, down, up) and the linear-scan open list picks the
// first node with minimum f.
// ----------------------------------------------------------------------

#[test]
fn a_star_step_in_open_grid_picks_right_neighbour_first() {
    // With Manhattan heuristic and 4-neighbour A* on a free grid, the
    // first step is the right neighbour (dirs = [[1,0],[-1,0],[0,1],[0,-1]]).
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1, 1], &[1, 1, 1], &[1, 1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 2i64, 2i64, cells))
        .unwrap();
    assert_eq!(xy(result), (1, 0));
}

// ----------------------------------------------------------------------
// Failure modes
// ----------------------------------------------------------------------

#[test]
fn a_star_step_returns_minus_one_when_no_path_exists() {
    // Disconnected grid: top-left is isolated from bottom-right.
    let rt = load_astar_script();
    let cells = grid(&[&[1, 0, 0], &[0, 0, 0], &[0, 0, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 2i64, 2i64, cells))
        .unwrap();
    assert_eq!(xy(result), (-1, -1));
}

#[test]
fn a_star_step_returns_minus_one_when_start_is_blocked() {
    let rt = load_astar_script();
    let cells = grid(&[&[0, 1], &[1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 1i64, 1i64, cells))
        .unwrap();
    assert_eq!(xy(result), (-1, -1));
}

#[test]
fn a_star_step_returns_minus_one_when_goal_is_blocked() {
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1], &[1, 0]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 1i64, 1i64, cells))
        .unwrap();
    assert_eq!(xy(result), (-1, -1));
}

#[test]
fn a_star_step_returns_minus_one_when_goal_is_out_of_bounds() {
    let rt = load_astar_script();
    let cells = grid(&[&[1, 1], &[1, 1]]);
    let result: Array = rt
        .call_fn("a_star_step", (0i64, 0i64, 5i64, 5i64, cells))
        .unwrap();
    assert_eq!(xy(result), (-1, -1));
}
