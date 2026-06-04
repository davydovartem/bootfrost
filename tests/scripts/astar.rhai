// A* pathfinding on a 2D grid for Bootfrost's block-move planning.
//
// The script exposes two functions, both invoked from PCSF as
//     rhai_call("a_star_step",         [x, y, u, v, cells])
//     rhai_call("a_star_path_exists",  [x, y, u, v, cells])
// (the first argument to `rhai_call` is the function name, the second
// is the Array of arguments forwarded to the script).
//
// `cells` is a 2D array indexed `cells[row][col]`. A value of 0 marks
// a blocked cell, any non-zero value is free. The caller is
// responsible for ensuring the array is rectangular.
//
// `a_star_step` returns the 2-element array [next_x, next_y] — the
// first cell on the shortest 4-neighbour path from (x, y) to (u, v).
// When the start and goal coincide it returns [x, y]. When the start
// is blocked, the target is unreachable, or the inputs are out of
// range, it returns [-1, -1].
//
// `a_star_path_exists` is a thin wrapper that returns the integer 1
// when a non-trivial path exists and 0 otherwise. PCSF's `==`
// comparison on integer terms works through term interning, so the
// formula can guard movement with
//     rhai_call("a_star_path_exists", [...]) == 1
// to avoid passing [-1, -1] into the cells-matrix `set(...)` and
// panicking on an out-of-bounds index.

fn a_star_step(args) {
    if len(args) != 5 { return [-1, -1]; }
    let x = args[0];
    let y = args[1];
    let u = args[2];
    let v = args[3];
    let cells = args[4];

    if !in_bounds(x, y, cells) { return [-1, -1]; }
    if !in_bounds(u, v, cells) { return [-1, -1]; }
    if x == u && y == v { return [x, y]; }
    return search(x, y, u, v, cells);
}

fn a_star_path_exists(args) {
    let r = a_star_step(args);
    if r[0] < 0 || r[1] < 0 { return 0; }
    if r[0] == 0 && r[1] == 0 && len(args) == 5 {
        // Disambiguate the "no path" sentinel [-1, -1] from a real
        // path that happens to land on the origin.  Only return 1 if
        // a_star_step itself signalled a valid (non -1) answer.
    }
    if r[0] == -1 { return 0; }
    return 1;
}

fn in_bounds(px, py, cells) {
    if len(cells) == 0 { return false; }
    if py < 0 || py >= len(cells) { return false; }
    if px < 0 || px >= len(cells[py]) { return false; }
    return cells[py][px] != 0;
}

/// Return the index of the element with the smallest f-score (item[0]).
/// Rhai passes arrays by value, so the actual `remove` must be performed
/// by the caller on its own copy of the open list.
fn find_min(open) {
    let best_i = 0;
    let best_f = open[0][0];
    let n = len(open);
    for i in 0..n {
        if open[i][0] < best_f {
            best_f = open[i][0];
            best_i = i;
        }
    }
    return best_i;
}

fn heuristic(px, py, u, v) {
    return abs(px - u) + abs(py - v);
}

fn search(x, y, u, v, cells) {
    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    let rows = len(cells);
    let cols = len(cells[0]);

    let open = [];
    let parent_map = #{};
    parent_map[`${x},${y}`] = [-1, -1];
    open.push([heuristic(x, y, u, v), 0, x, y]);

    let max_iter = rows * cols * 4 + 16;
    let iters = 0;

    while len(open) > 0 && iters < max_iter {
        iters += 1;
        let best_i = find_min(open);
        let node = open.remove(best_i);
        let cg = node[1];
        let cx = node[2];
        let cy = node[3];

        if cx == u && cy == v {
            // Reconstruct by walking parent_map. The start has parent
            // (-1, -1) as a sentinel, so we stop when we hit that.
            let path = [[cx, cy]];
            let cur_x = cx;
            let cur_y = cy;
            let safety = 0;
            while safety < 100000 {
                safety += 1;
                let p = parent_map[`${cur_x},${cur_y}`];
                if p[0] < 0 { break; }
                path.insert(0, [p[0], p[1]]);
                cur_x = p[0];
                cur_y = p[1];
            }
            if len(path) > 1 {
                return path[1];
            }
            return [cx, cy];
        }

        for d in dirs {
            let nx = cx + d[0];
            let ny = cy + d[1];
            if ny < 0 || ny >= rows { continue; }
            if nx < 0 || nx >= len(cells[ny]) { continue; }
            if cells[ny][nx] == 0 { continue; }
            if parent_map.contains(`${nx},${ny}`) { continue; }

            let ng = cg + 1;
            let nf = ng + heuristic(nx, ny, u, v);
            parent_map[`${nx},${ny}`] = [cx, cy];
            open.push([nf, ng, nx, ny]);
        }
    }

    return [-1, -1];
}

