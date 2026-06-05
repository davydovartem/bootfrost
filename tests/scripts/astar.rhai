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

fn invalid_step() {
    return [-1, -1];
}

fn in_bounds(x, y, width, height) {
    return x >= 0 && y >= 0 && x < width && y < height;
}

fn cell_at(cells, x, y) {
    let row = cells[y];
    return row[x];
}

fn matrix_get(matrix, x, y) {
    let row = matrix[y];
    return row[x];
}

fn matrix_set(matrix, x, y, value) {
    let row = matrix[y];
    row[x] = value;
    matrix[y] = row;
    return matrix;
}

fn make_matrix(height, width, value) {
    let matrix = [];
    let y = 0;
    while y < height {
        let row = [];
        let x = 0;
        while x < width {
            row.push(value);
            x += 1;
        }
        matrix.push(row);
        y += 1;
    }
    return matrix;
}

fn remove_index(arr, idx) {
    let result = [];
    let i = 0;
    while i < arr.len() {
        if i != idx {
            result.push(arr[i]);
        }
        i += 1;
    }
    return result;
}

fn heuristic(x, y, u, v) {
    return abs(x - u) + abs(y - v);
}

fn reconstruct_first_step(start_x, start_y, goal_x, goal_y, prev_x, prev_y) {
    let cx = goal_x;
    let cy = goal_y;

    while true {
        let px = matrix_get(prev_x, cx, cy);
        let py = matrix_get(prev_y, cx, cy);

        if px == -1 || py == -1 {
            return invalid_step();
        }

        if px == start_x && py == start_y {
            return [cx, cy];
        }

        cx = px;
        cy = py;
    }
}

fn a_star_step(args) {
    if args.len() != 5 {
        return invalid_step();
    }

    let start_x = args[0];
    let start_y = args[1];
    let goal_x = args[2];
    let goal_y = args[3];
    let cells = args[4];

    let height = cells.len();
    if height == 0 {
        return invalid_step();
    }

    let first_row = cells[0];
    let width = first_row.len();
    if width == 0 {
        return invalid_step();
    }

    if !in_bounds(start_x, start_y, width, height) || !in_bounds(goal_x, goal_y, width, height) {
        return invalid_step();
    }

    if cell_at(cells, goal_x, goal_y) == 0 {
        return invalid_step();
    }

    if start_x == goal_x && start_y == goal_y {
        return [start_x, start_y];
    }

    let inf = width * height + 1;
    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];

    let open = [[start_x, start_y]];
    let open_flags = matrix_set(make_matrix(height, width, false), start_x, start_y, true);
    let closed = make_matrix(height, width, false);
    let g_score = matrix_set(make_matrix(height, width, inf), start_x, start_y, 0);
    let f_score = matrix_set(
        make_matrix(height, width, inf),
        start_x,
        start_y,
        heuristic(start_x, start_y, goal_x, goal_y)
    );
    let prev_x = make_matrix(height, width, -1);
    let prev_y = make_matrix(height, width, -1);

    while open.len() > 0 {
        let best_idx = 0;
        let first = open[0];
        let best_x = first[0];
        let best_y = first[1];
        let best_f = matrix_get(f_score, best_x, best_y);

        let i = 1;
        while i < open.len() {
            let node = open[i];
            let node_x = node[0];
            let node_y = node[1];
            let node_f = matrix_get(f_score, node_x, node_y);
            if node_f < best_f {
                best_idx = i;
                best_x = node_x;
                best_y = node_y;
                best_f = node_f;
            }
            i += 1;
        }

        open = remove_index(open, best_idx);
        open_flags = matrix_set(open_flags, best_x, best_y, false);

        if best_x == goal_x && best_y == goal_y {
            return reconstruct_first_step(start_x, start_y, goal_x, goal_y, prev_x, prev_y);
        }

        if matrix_get(closed, best_x, best_y) {
            continue;
        }
        closed = matrix_set(closed, best_x, best_y, true);

        let current_g = matrix_get(g_score, best_x, best_y);

        let d = 0;
        while d < dirs.len() {
            let dir = dirs[d];
            let next_x = best_x + dir[0];
            let next_y = best_y + dir[1];

            if in_bounds(next_x, next_y, width, height)
                && cell_at(cells, next_x, next_y) != 0
                && !matrix_get(closed, next_x, next_y) {
                let tentative_g = current_g + 1;
                if tentative_g < matrix_get(g_score, next_x, next_y) {
                    prev_x = matrix_set(prev_x, next_x, next_y, best_x);
                    prev_y = matrix_set(prev_y, next_x, next_y, best_y);
                    g_score = matrix_set(g_score, next_x, next_y, tentative_g);
                    f_score = matrix_set(
                        f_score,
                        next_x,
                        next_y,
                        tentative_g + heuristic(next_x, next_y, goal_x, goal_y)
                    );

                    if !matrix_get(open_flags, next_x, next_y) {
                        open.push([next_x, next_y]);
                        open_flags = matrix_set(open_flags, next_x, next_y, true);
                    }
                }
            }

            d += 1;
        }
    }

    return invalid_step();
}

fn a_star_path_exists(args) {
    let step = a_star_step(args);
    print(`step: args: ${args[0]}, ${args[1]}, ${args[2]}, ${args[3]}`);
    if step[0] >= 0 && (step[0] != args[0] || step[1] != args[1]) {
        return 1;
    }
    return 0;
}
