from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tools.block_move_viewer.viewer import (  # noqa: E402
    BLOCK_IDS,
    FREE_FILL,
    GRID_LINE,
    OCCUPIED_FILL,
    ROBOT_FILL,
    TARGET_FILL,
    TARGET_OUTLINE,
    SceneState,
    load_frames,
)


TEXT = "#111827"
MUTED = "#4b5563"
FIG_BG = "#ffffff"
GRID_BG = "#eef2f7"
PANEL_BG = "#f8fafc"
PANEL_BORDER = "#d1d5db"
ROBOT_COLORS = {
    "1": "#2563eb",
    "2": "#0f766e",
    "3": "#7c3aed",
}
BLOCK_COLORS = {
    "B1": "#d97706",
    "B2": "#dc2626",
}
ENTITY_ORDER = ["1", "2", "3", "B1", "B2"]
KEY_STEPS = [0, 21, 37, 78, 95, 97]


@dataclass(frozen=True)
class Layout:
    cell: int
    margin_left: int
    margin_right: int
    margin_top: int
    margin_bottom: int


MAIN_LAYOUT = Layout(cell=72, margin_left=74, margin_right=42, margin_top=52, margin_bottom=66)
INSET_LAYOUT = Layout(cell=28, margin_left=34, margin_right=18, margin_top=26, margin_bottom=32)


def load_font(size: int, *, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = ["arialbd.ttf", "DejaVuSans-Bold.ttf"] if bold else ["arial.ttf", "DejaVuSans.ttf"]
    for candidate in candidates:
        try:
            return ImageFont.truetype(candidate, size=size)
        except OSError:
            continue
    return ImageFont.load_default()


TITLE_FONT = load_font(36, bold=True)
SECTION_FONT = load_font(24, bold=True)
TEXT_FONT = load_font(18)
SMALL_FONT = load_font(15)
TINY_FONT = load_font(13)


def select_export_frames(json_path: Path) -> tuple[list, set[tuple[int, int]]]:
    frames, goal_cells = load_frames(json_path)
    selected = [frame for frame in frames if frame.step_number is not None and 0 <= frame.step_number <= 97]
    if not selected:
        raise ValueError("Failed to find exported step frames 0..97 in JSON log")
    return selected, goal_cells


def cell_box(x: int, y: int, state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> tuple[int, int, int, int]:
    left = origin[0] + layout.margin_left + x * layout.cell
    top = origin[1] + layout.margin_top + (state.height - 1 - y) * layout.cell
    return left, top, left + layout.cell, top + layout.cell


def robot_center(position: tuple[int, int], state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> tuple[float, float]:
    box = cell_box(position[0], position[1], state, layout, origin)
    return ((box[0] + box[2]) / 2.0, (box[1] + box[3]) / 2.0)


def block_center(position: tuple[int, int], state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> tuple[float, float]:
    left = cell_box(position[0], position[1], state, layout, origin)
    right = cell_box(position[0] + 1, position[1], state, layout, origin)
    return ((left[0] + right[2]) / 2.0, (left[1] + right[3]) / 2.0)


def unique_positions(frames: list, entity: str) -> list[tuple[int, int]]:
    positions: list[tuple[int, int]] = []
    previous = object()
    for frame in frames:
        position = frame.state.positions.get(entity)
        if position is None:
            continue
        if position != previous:
            positions.append(position)
            previous = position
    return positions


def entity_color(entity: str) -> str:
    if entity in ROBOT_COLORS:
        return ROBOT_COLORS[entity]
    return BLOCK_COLORS[entity]


def scene_size(state: SceneState, layout: Layout) -> tuple[int, int]:
    width = layout.margin_left + state.width * layout.cell + layout.margin_right
    height = layout.margin_top + state.height * layout.cell + layout.margin_bottom
    return width, height


def draw_arrow(draw: ImageDraw.ImageDraw, center: tuple[float, float], direction: str, *, scale: float = 1.0) -> None:
    cx, cy = center
    shaft = 14 * scale
    head = 7 * scale
    if direction == "N":
        points = [(cx, cy - shaft), (cx - head, cy), (cx - 2 * scale, cy), (cx - 2 * scale, cy + shaft), (cx + 2 * scale, cy + shaft), (cx + 2 * scale, cy), (cx + head, cy)]
    elif direction == "E":
        points = [(cx + shaft, cy), (cx, cy - head), (cx, cy - 2 * scale), (cx - shaft, cy - 2 * scale), (cx - shaft, cy + 2 * scale), (cx, cy + 2 * scale), (cx, cy + head)]
    elif direction == "S":
        points = [(cx, cy + shaft), (cx - head, cy), (cx - 2 * scale, cy), (cx - 2 * scale, cy - shaft), (cx + 2 * scale, cy - shaft), (cx + 2 * scale, cy), (cx + head, cy)]
    else:
        points = [(cx - shaft, cy), (cx, cy - head), (cx, cy - 2 * scale), (cx + shaft, cy - 2 * scale), (cx + shaft, cy + 2 * scale), (cx, cy + 2 * scale), (cx, cy + head)]
    draw.polygon(points, fill="#ffffff")


def rgba(hex_color: str, alpha: int) -> tuple[int, int, int, int]:
    hex_color = hex_color.lstrip("#")
    return tuple(int(hex_color[i : i + 2], 16) for i in (0, 2, 4)) + (alpha,)


def draw_base_scene(
    image: Image.Image,
    state: SceneState,
    layout: Layout,
    goal_cells: set[tuple[int, int]],
    *,
    grid_labels: bool,
    panel_box: tuple[int, int, int, int] | None = None,
    origin: tuple[int, int] = (0, 0),
) -> None:
    draw = ImageDraw.Draw(image)
    if panel_box is not None:
        draw.rounded_rectangle(panel_box, radius=24, fill=PANEL_BG, outline=PANEL_BORDER, width=2)
    for x in range(state.width):
        for y in range(state.height):
            box = cell_box(x, y, state, layout, origin)
            free_value = state.free_cells.get((x, y), 0)
            fill = FREE_FILL if free_value == 1 else OCCUPIED_FILL
            draw.rectangle(box, fill=fill, outline=GRID_LINE, width=1)
    for x, y in goal_cells:
        if 0 <= x < state.width and 0 <= y < state.height:
            draw.rounded_rectangle(cell_box(x, y, state, layout, origin), radius=max(6, layout.cell // 9), fill=TARGET_FILL, outline=TARGET_OUTLINE, width=2)
    if grid_labels:
        for x in range(state.width):
            box = cell_box(x, 0, state, layout, origin)
            draw.text((box[0] + layout.cell / 2 - 5, origin[1] + layout.margin_top + state.height * layout.cell + 10), str(x), fill=MUTED, font=SMALL_FONT)
        for y in range(state.height):
            box = cell_box(0, y, state, layout, origin)
            draw.text((origin[0] + layout.margin_left - 32, box[1] + layout.cell / 2 - 8), str(y), fill=MUTED, font=SMALL_FONT)


def draw_block(draw: ImageDraw.ImageDraw, entity: str, position: tuple[int, int], state: SceneState, layout: Layout, *, outline: str | None = None, alpha: int = 255, label: bool = True, origin: tuple[int, int] = (0, 0)) -> None:
    left = cell_box(position[0], position[1], state, layout, origin)
    right = cell_box(position[0] + 1, position[1], state, layout, origin)
    box = (left[0] + 3, left[1] + max(5, layout.cell // 8), right[2] - 3, right[3] - max(5, layout.cell // 8))
    fill = rgba(entity_color(entity), alpha) if alpha < 255 else entity_color(entity)
    border = outline or "#7c2d12"
    draw.rounded_rectangle(box, radius=max(8, layout.cell // 6), fill=fill, outline=border, width=2)
    if label:
        text_box = draw.textbbox((0, 0), entity, font=TEXT_FONT if layout.cell >= 50 else TINY_FONT)
        text_x = (box[0] + box[2] - (text_box[2] - text_box[0])) / 2
        text_y = (box[1] + box[3] - (text_box[3] - text_box[1])) / 2 - 1
        draw.text((text_x, text_y), entity, fill="#ffffff", font=TEXT_FONT if layout.cell >= 50 else TINY_FONT)


def draw_robot(draw: ImageDraw.ImageDraw, entity: str, position: tuple[int, int], direction: str, state: SceneState, layout: Layout, *, alpha: int = 255, label: bool = True, show_direction: bool = True, origin: tuple[int, int] = (0, 0)) -> None:
    cx, cy = robot_center(position, state, layout, origin)
    radius = layout.cell * 0.28
    fill = rgba(entity_color(entity), alpha) if alpha < 255 else entity_color(entity)
    outline = rgba(entity_color(entity), min(255, alpha + 10)) if alpha < 255 else entity_color(entity)
    draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=fill, outline=outline, width=2)
    if show_direction:
        draw_arrow(draw, (cx, cy), direction, scale=layout.cell / 56.0)
    if label:
        text = f"R{entity}"
        bbox = draw.textbbox((0, 0), text, font=SMALL_FONT if layout.cell >= 50 else TINY_FONT)
        draw.text((cx - (bbox[2] - bbox[0]) / 2, cy + radius + 4), text, fill=TEXT, font=SMALL_FONT if layout.cell >= 50 else TINY_FONT)


def draw_paths(image: Image.Image, frames: list, state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> None:
    draw = ImageDraw.Draw(image, "RGBA")
    for entity in ENTITY_ORDER:
        positions = unique_positions(frames, entity)
        if len(positions) < 2:
            continue
        if entity in BLOCK_IDS:
            points = [block_center(position, state, layout, origin) for position in positions]
            width = max(6, layout.cell // 8)
        else:
            points = [robot_center(position, state, layout, origin) for position in positions]
            width = max(5, layout.cell // 10)
        color = entity_color(entity)
        draw.line(points, fill=rgba(color, 190), width=width, joint="curve")
        for point in points[1:-1]:
            r = max(4, width // 2)
            draw.ellipse((point[0] - r, point[1] - r, point[0] + r, point[1] + r), fill=rgba(color, 180))


def draw_start_end(image: Image.Image, frames: list, state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> None:
    draw = ImageDraw.Draw(image)
    start_frame = frames[0]
    end_frame = frames[-1]
    for entity in ENTITY_ORDER:
        start_pos = start_frame.state.positions.get(entity)
        end_pos = end_frame.state.positions.get(entity)
        if start_pos is not None:
            if entity in BLOCK_IDS:
                draw_block(draw, entity, start_pos, state, layout, outline=entity_color(entity), alpha=110, origin=origin)
            else:
                draw_robot(draw, entity, start_pos, start_frame.state.robot_dirs.get(entity, "N"), state, layout, alpha=110, origin=origin)
        if end_pos is not None:
            if entity in BLOCK_IDS:
                draw_block(draw, entity, end_pos, state, layout, outline="#111827", origin=origin)
            else:
                draw_robot(draw, entity, end_pos, end_frame.state.robot_dirs.get(entity, "N"), state, layout, origin=origin)


def draw_trail_footprints(image: Image.Image, frames: list, state: SceneState, layout: Layout, origin: tuple[int, int] = (0, 0)) -> None:
    overlay = Image.new("RGBA", image.size, (255, 255, 255, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")
    for entity in ENTITY_ORDER:
        positions = unique_positions(frames, entity)
        for index, position in enumerate(positions):
            alpha = min(160, 40 + index * 6)
            if entity in BLOCK_IDS:
                draw_block(draw, entity, position, state, layout, outline=entity_color(entity), alpha=alpha, label=False, origin=origin)
            else:
                draw_robot(draw, entity, position, frames[0].state.robot_dirs.get(entity, "N"), state, layout, alpha=alpha, label=False, show_direction=False, origin=origin)
    image.alpha_composite(overlay)


def add_legend(image: Image.Image, origin: tuple[int, int]) -> None:
    draw = ImageDraw.Draw(image)
    x, y = origin
    draw.text((x, y), "Trajectories", fill=TEXT, font=SECTION_FONT)
    y += 36
    for entity in ENTITY_ORDER:
        color = entity_color(entity)
        draw.rounded_rectangle((x, y + 5, x + 22, y + 21), radius=5, fill=color, outline=color)
        label = f"Robot {entity}" if entity in ROBOT_COLORS else f"Block {entity}"
        draw.text((x + 34, y), label, fill=TEXT, font=TEXT_FONT)
        y += 30
    draw.rounded_rectangle((x, y + 7, x + 22, y + 23), radius=5, fill="#ffffff", outline=TARGET_OUTLINE, width=2)
    draw.text((x + 34, y), "Goal area", fill=TEXT, font=TEXT_FONT)
    y += 30
    draw.text((x, y + 6), "Transparent figures: start / prev", fill=MUTED, font=SMALL_FONT)
    draw.text((x, y + 28), "Saturated figures: finish", fill=MUTED, font=SMALL_FONT)


def render_main_trajectory_panel(frames: list, goal_cells: set[tuple[int, int]]) -> Image.Image:
    state = frames[0].state
    scene_w, scene_h = scene_size(state, MAIN_LAYOUT)
    legend_w = 310
    image = Image.new("RGBA", (scene_w + legend_w + 64, scene_h + 84), FIG_BG)
    draw = ImageDraw.Draw(image)
    draw.text((40, 24), "Blocks movement plan: trajectories on the scene", fill=TEXT, font=TITLE_FONT)
    origin = (24, 74)
    panel_box = (origin[0], origin[1], origin[0] + scene_w, origin[1] + scene_h)
    draw_base_scene(image, state, MAIN_LAYOUT, goal_cells, grid_labels=True, panel_box=panel_box, origin=origin)
    draw_paths(image, frames, state, MAIN_LAYOUT, origin)
    draw_start_end(image, frames, state, MAIN_LAYOUT, origin)
    add_legend(image, (scene_w + 46, 108))
    return image


def render_overlay_figure(frames: list, goal_cells: set[tuple[int, int]]) -> Image.Image:
    state = frames[0].state
    scene_w, scene_h = scene_size(state, MAIN_LAYOUT)
    legend_w = 330
    image = Image.new("RGBA", (scene_w + legend_w + 76, scene_h + 84), FIG_BG)
    draw = ImageDraw.Draw(image)
    draw.text((40, 24), "Blocks movement plan: the trace of movements across the scene", fill=TEXT, font=TITLE_FONT)
    origin = (24, 74)
    panel_box = (origin[0], origin[1], origin[0] + scene_w, origin[1] + scene_h)
    draw_base_scene(image, state, MAIN_LAYOUT, goal_cells, grid_labels=True, panel_box=panel_box, origin=origin)
    draw_trail_footprints(image, frames, state, MAIN_LAYOUT, origin)
    draw_paths(image, frames, state, MAIN_LAYOUT, origin)
    draw_start_end(image, frames, state, MAIN_LAYOUT, origin)
    add_legend(image, (scene_w + 48, 108))
    draw.text((scene_w + 48, scene_h - 34), "The trace is built based on the frames of the visualizer, steps from 0 to 97.", fill=MUTED, font=SMALL_FONT)
    return image


def render_key_states_figure(frames: list, goal_cells: set[tuple[int, int]]) -> Image.Image:
    top = render_main_trajectory_panel(frames, goal_cells)
    state = frames[0].state
    inset_w, inset_h = scene_size(state, INSET_LAYOUT)
    gap = 18
    title_h = 72
    total_w = max(top.width, 32 + len(KEY_STEPS) * inset_w + (len(KEY_STEPS) - 1) * gap + 32)
    total_h = top.height + title_h + inset_h + 44
    image = Image.new("RGBA", (total_w, total_h), FIG_BG)
    image.paste(top, ((total_w - top.width) // 2, 0))
    draw = ImageDraw.Draw(image)
    draw.text((32, top.height + 12), "Key states of a successful plan", fill=TEXT, font=SECTION_FONT)
    by_step = {frame.step_number: frame for frame in frames}
    x = 32
    y = top.height + title_h
    for step in KEY_STEPS:
        frame = by_step[step]
        panel_box = (x, y, x + inset_w, y + inset_h)
        draw_base_scene(image, frame.state, INSET_LAYOUT, goal_cells, grid_labels=False, panel_box=panel_box, origin=(x, y))
        inset_draw = ImageDraw.Draw(image)
        for block in frame.state.blocks:
            position = frame.state.positions.get(block)
            if position is not None:
                draw_block(inset_draw, block, position, frame.state, INSET_LAYOUT, origin=(x, y))
        for robot in frame.state.robots:
            position = frame.state.positions.get(robot)
            if position is not None:
                draw_robot(inset_draw, robot, position, frame.state.robot_dirs.get(robot, "N"), frame.state, INSET_LAYOUT, origin=(x, y))
        caption = f"шаг {step}"
        bbox = inset_draw.textbbox((0, 0), caption, font=TEXT_FONT)
        inset_draw.text((x + (inset_w - (bbox[2] - bbox[0])) / 2, y + inset_h + 8), caption, fill=MUTED, font=TEXT_FONT)
        x += inset_w + gap
    return image


def save_figures(json_path: Path, output_dir: Path) -> list[Path]:
    frames, goal_cells = select_export_frames(json_path)
    output_dir.mkdir(parents=True, exist_ok=True)
    key_states_path = output_dir / "block_move_planning_trajectory_key_states.png"
    trail_path = output_dir / "block_move_planning_trajectory_trail.png"
    render_key_states_figure(frames, goal_cells).convert("RGB").save(key_states_path)
    render_overlay_figure(frames, goal_cells).convert("RGB").save(trail_path)
    return [key_states_path, trail_path]


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate article-ready figures for BlockMovePlanning steps.")
    parser.add_argument(
        "--json",
        type=Path,
        default=ROOT / "problems" / "BlockMovePlanning" / "BlockMovePlanning.json",
        help="Path to BlockMovePlanning JSON log.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "papers" / "Article2026" / "figures",
        help="Output directory for generated figures.",
    )
    args = parser.parse_args()

    for path in save_figures(args.json, args.out):
        print(path)


if __name__ == "__main__":
    main()
