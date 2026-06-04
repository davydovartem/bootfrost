// A* pathfinding on a 2D grid for Bootfrost's block-move planning.
//
// Scenario: on each step of the inference the prover asks the script
// for the next cell to move to. The function below returns the first
// step of the shortest 4-neighbour path from (x, y) to (u, v), avoiding
// cells whose value in `cells` is 0.
//
// `cells` is a 2D array with cells[row][col] indexing. A value of 0
// marks a blocked cell, any non-zero value is free. The caller is
// responsible for ensuring the array is rectangular.
//
// Returns:
//   [next_x, next_y]   if a path exists: the first step after (x, y).
//   [x, y]              if start and goal coincide.
//   [-1, -1]            if no path exists or inputs are out of range.

fn a_star_step(x, y, u, v, cells) {
    if !in_bounds(x, y, cells) { return [-1, -1]; }
    if !in_bounds(u, v, cells) { return [-1, -1]; }
    if x == u && y == v { return [x, y]; }
    return search(x, y, u, v, cells);
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
