// Verification script: a small library of grid-distance metrics that the
// Bootfrost prover can invoke from PCSF formulas as IFunctor equivalents.
// Loaded by the user via `RhaiRuntime::load_file`.
//
// All functions take four i64 arguments (x1, y1, x2, y2) and return i64.
// The coordinates are grid cells: positive integers are valid cell indices,
// but the functions are written so that they also handle negative offsets
// (e.g. for vector deltas that may straddle zero).

/// Manhattan (L1) distance between (x1, y1) and (x2, y2):
/// the number of 4-neighbour steps required to walk from one point to
/// the other on an empty grid.
fn manhattan(x1, y1, x2, y2) {
    return abs(x1 - x2) + abs(y1 - y2);
}

/// Chebyshev (L-infinity) distance: the number of 8-neighbour (king-move)
/// steps required on an empty grid.
fn chebyshev(x1, y1, x2, y2) {
    let dx = abs(x1 - x2);
    let dy = abs(y1 - y2);
    if dx > dy { return dx; }
    return dy;
}

/// Squared Euclidean distance. Avoids `sqrt` and stays in i64, which is
/// convenient when the caller only needs to compare distances, not to
/// read off a real-valued magnitude.
fn euclidean_sq(x1, y1, x2, y2) {
    let dx = x1 - x2;
    let dy = y1 - y2;
    return dx * dx + dy * dy;
}
