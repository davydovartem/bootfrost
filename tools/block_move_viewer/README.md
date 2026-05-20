# Block Move Viewer

Python viewer for `problems/BlockMovePlanning.json`.

Features:

- reads the initial base from `formula.children[0].atoms_list`
- steps through solver log entries from `log[*]`
- shows atoms added on the step
- shows atoms used on the step
- shows atoms removed from the active base compared to the previous step
- renders the current scene state reconstructed from the active `base`
- highlights the target area extracted from goal `Pos(B*, x, y)` atoms
- saves the current frame as a PNG image
- supports keyboard navigation with Left/Right arrows

## Install

```bash
python -m pip install -r tools/block_move_viewer/requirements.txt
```

## Run

```bash
python tools/block_move_viewer/viewer.py
```

Custom log path:

```bash
python tools/block_move_viewer/viewer.py --json problems/BlockMovePlanning.json
```

## Notes

- robots are drawn as circles with direction arrows
- blocks are drawn as horizontal 2-cell objects
- `Pos(B1, x, y)` means the left cell of the block is at `(x, y)`
- `Free(x, y, 1)` is shown as a free cell, `Free(x, y, 0)` as occupied
