//! Integration tests for `RhaiRuntime` against a real `.rhai` file on disk.
//!
//! These tests live in `tests/` (a separate crate as far as Cargo is
//! concerned) and only touch the public API. They verify that the runtime
//! can load a script from the filesystem, expose every function defined
//! in it, and report diagnostic errors for missing functions.
//!
//! The fixture script lives at `tests/scripts/manhattan.rhai`. The path is
//! resolved relative to `CARGO_MANIFEST_DIR` so the tests work from any
//! working directory and under `cargo test`.

use bootfrost::strategies::rhai_runtime::{RhaiRuntime, RhaiRuntimeError};

/// Absolute path to the bundled verification script.
fn manhattan_script_path() -> String {
    format!(
        "{}/tests/scripts/manhattan.rhai",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Open the bundled verification script and return a runtime.
/// Panics with a clear message if the file is missing, so a stale
/// test fixture is reported loudly rather than as a confusing
/// `RhaiRuntimeError::FileNotFound`.
fn load_manhattan_script() -> RhaiRuntime {
    let path = manhattan_script_path();
    RhaiRuntime::from_file(&path)
        .unwrap_or_else(|e| panic!("failed to load '{}': {}", path, e))
}

#[test]
fn manhattan_script_loads_and_source_path_is_recorded() {
    let rt = load_manhattan_script();
    assert!(rt.is_loaded());
    let path = rt
        .source_path()
        .expect("source_path should be set for file-backed scripts");
    assert!(
        path.ends_with("manhattan.rhai"),
        "unexpected source_path: {}",
        path
    );
}

#[test]
fn manhattan_distance_for_3_4_5_triangle() {
    let rt = load_manhattan_script();
    // (0,0) -> (3,4): |3| + |4| = 7
    let d: i64 = rt.call_fn("manhattan", (0i64, 0i64, 3i64, 4i64)).unwrap();
    assert_eq!(d, 7);
}

#[test]
fn manhattan_distance_handles_negative_coordinates() {
    let rt = load_manhattan_script();
    // (-2, -3) -> (1, 1): |3| + |4| = 7
    let d: i64 = rt.call_fn("manhattan", (-2i64, -3i64, 1i64, 1i64)).unwrap();
    assert_eq!(d, 7);
}

#[test]
fn manhattan_distance_is_symmetric() {
    let rt = load_manhattan_script();
    let forward: i64 = rt.call_fn("manhattan", (0i64, 0i64, 3i64, 4i64)).unwrap();
    let backward: i64 = rt.call_fn("manhattan", (3i64, 4i64, 0i64, 0i64)).unwrap();
    assert_eq!(forward, backward);
}

#[test]
fn manhattan_distance_is_zero_for_same_point() {
    let rt = load_manhattan_script();
    let d: i64 = rt.call_fn("manhattan", (5i64, 5i64, 5i64, 5i64)).unwrap();
    assert_eq!(d, 0);
}

#[test]
fn chebyshev_distance_takes_max_of_axis_deltas() {
    let rt = load_manhattan_script();
    // (0,0) -> (3,4): max(|3|, |4|) = 4
    let d: i64 = rt.call_fn("chebyshev", (0i64, 0i64, 3i64, 4i64)).unwrap();
    assert_eq!(d, 4);
}

#[test]
fn euclidean_squared_matches_3_4_5_triangle() {
    let rt = load_manhattan_script();
    // (0,0) -> (3,4): 3^2 + 4^2 = 25
    let d: i64 = rt.call_fn("euclidean_sq", (0i64, 0i64, 3i64, 4i64)).unwrap();
    assert_eq!(d, 25);
}

#[test]
fn same_runtime_can_call_multiple_functions_defined_in_one_script() {
    // Verifies that loading a single file exposes every function in it,
    // i.e. the runtime is reusable across calls.
    let rt = load_manhattan_script();
    let m: i64 = rt.call_fn("manhattan", (0i64, 0i64, 1i64, 1i64)).unwrap();
    let c: i64 = rt.call_fn("chebyshev", (0i64, 0i64, 1i64, 1i64)).unwrap();
    let e: i64 = rt.call_fn("euclidean_sq", (0i64, 0i64, 1i64, 1i64)).unwrap();
    assert_eq!(m, 2);
    assert_eq!(c, 1);
    assert_eq!(e, 2);
}

#[test]
fn unknown_function_from_real_script_returns_function_not_found() {
    let rt = load_manhattan_script();
    let err: RhaiRuntimeError = rt
        .call_fn::<i64, _>("minkowski", (0i64, 0i64, 1i64, 1i64))
        .unwrap_err();
    match err {
        RhaiRuntimeError::FunctionNotFound { name, .. } => {
            assert_eq!(name, "minkowski");
        }
        other => panic!("expected FunctionNotFound, got {:?}", other),
    }
}
