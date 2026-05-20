from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from typing import Any

try:
    from PIL import Image, ImageDraw, ImageFont, ImageTk
except ImportError as exc:  # pragma: no cover - runtime dependency guard
    raise SystemExit(
        "Pillow is required for this viewer.\n"
        "Install it with: python -m pip install -r tools/block_move_viewer/requirements.txt"
    ) from exc


GRID_BG = "#f7f7fb"
GRID_LINE = "#cfd4df"
BLOCK_FILL = "#d97706"
BLOCK_TEXT = "#ffffff"
ROBOT_FILL = "#2563eb"
ROBOT_TEXT = "#ffffff"
FREE_FILL = "#ffffff"
OCCUPIED_FILL = "#d4d7dd"
TARGET_FILL = (34, 197, 94, 68)
TARGET_OUTLINE = "#15803d"
USED_HIGHLIGHT = "#2563eb"
ADDED_HIGHLIGHT = "#f59e0b"
BOTH_HIGHLIGHT = "#7c3aed"
TEXT = "#111827"
MUTED = "#4b5563"
PANEL_BG = "#ffffff"
PANEL_BORDER = "#d1d5db"

CELL_SIZE = 56
MARGIN_LEFT = 56
MARGIN_RIGHT = 32
MARGIN_TOP = 64
MARGIN_BOTTOM = 56
PANEL_GAP = 28
PANEL_WIDTH = 560
LINE_HEIGHT = 22

ROBOT_IDS = {"R1", "R2", "R3"}
BLOCK_IDS = {"B1", "B2"}


def term_to_text(term: dict[str, Any]) -> str:
    name = str(term.get("name", ""))
    args = term.get("args") or []
    if not args:
        return name
    return f"{name}(" + ", ".join(term_to_text(arg) for arg in args) + ")"


def atom_to_text(atom: dict[str, Any]) -> str:
    return f"{atom.get('name', '')}(" + ", ".join(term_to_text(arg) for arg in atom.get("args", [])) + ")"


def is_int_text(value: str) -> bool:
    if not value:
        return False
    if value.startswith("-"):
        return value[1:].isdigit()
    return value.isdigit()


def atom_args(atom: dict[str, Any]) -> list[str]:
    return [term_to_text(arg) for arg in atom.get("args", [])]


def extract_initial_atoms(data: dict[str, Any]) -> list[dict[str, Any]]:
    formula = data.get("formula", {})
    children = formula.get("children") or []
    if not children:
        raise ValueError("JSON does not contain formula.children")
    root_exists = children[0]
    atoms = root_exists.get("atoms_list") or []
    if not atoms:
        raise ValueError("Initial atoms were not found in formula.children[0].atoms_list")
    return atoms


def extract_goal_cells(data: dict[str, Any]) -> set[tuple[int, int]]:
    formula = data.get("formula", {})
    children = formula.get("children") or []
    if not children:
        return set()
    root_exists = children[0]
    goal_cells: set[tuple[int, int]] = set()
    for child in root_exists.get("children") or []:
        vars_list = child.get("vars_list") or []
        atoms_list = child.get("atoms_list") or []
        if vars_list or not atoms_list:
            continue
        if not all(atom.get("name") == "Pos" for atom in atoms_list):
            continue
        parsed_positions: list[tuple[str, int, int]] = []
        for atom in atoms_list:
            args = atom_args(atom)
            if len(args) != 3:
                parsed_positions = []
                break
            entity, x_text, y_text = args
            if entity not in BLOCK_IDS or not is_int_text(x_text) or not is_int_text(y_text):
                parsed_positions = []
                break
            parsed_positions.append((entity, int(x_text), int(y_text)))
        if not parsed_positions:
            continue
        for _, x, y in parsed_positions:
            goal_cells.add((x, y))
            goal_cells.add((x + 1, y))
    return goal_cells


def active_atoms_from_base(base_entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [entry["atom"] for entry in base_entries if not entry.get("deleted", False)]


def infer_entities(atoms: list[dict[str, Any]]) -> set[str]:
    entities: set[str] = set()
    for atom in atoms:
        args = atom_args(atom)
        if not args:
            continue
        first = args[0]
        if first in ROBOT_IDS or first in BLOCK_IDS:
            entities.add(first)
        if atom.get("name") in {"Move"} and len(args) >= 2 and args[1] in BLOCK_IDS:
            entities.add(args[1])
    return entities


@dataclass(frozen=True)
class SceneState:
    width: int
    height: int
    positions: dict[str, tuple[int, int]]
    robot_dirs: dict[str, str]
    free_cells: dict[tuple[int, int], int]
    robots: list[str]
    blocks: list[str]


@dataclass(frozen=True)
class Frame:
    index: int
    label: str
    answer: str
    question: int | None
    added_atoms: list[dict[str, Any]]
    used_atoms: list[dict[str, Any]]
    removed_atoms: list[str]
    active_atoms: list[dict[str, Any]]
    state: SceneState
    added_entities: set[str]
    used_entities: set[str]


def build_state(active_atoms: list[dict[str, Any]]) -> SceneState:
    width = 0
    height = 0
    positions: dict[str, tuple[int, int]] = {}
    robot_dirs: dict[str, str] = {}
    free_cells: dict[tuple[int, int], int] = {}
    robots: set[str] = set()
    blocks: set[str] = set()

    for atom in active_atoms:
        name = atom.get("name")
        args = atom_args(atom)
        if name == "W" and len(args) == 1 and is_int_text(args[0]):
            width = int(args[0])
        elif name == "H" and len(args) == 1 and is_int_text(args[0]):
            height = int(args[0])
        elif name == "R" and len(args) == 2:
            robots.add(args[0])
            robot_dirs[args[0]] = args[1]
        elif name == "B" and len(args) == 1:
            blocks.add(args[0])
        elif name == "Pos" and len(args) == 3 and is_int_text(args[1]) and is_int_text(args[2]):
            positions[args[0]] = (int(args[1]), int(args[2]))
        elif name == "Free" and len(args) == 3 and all(is_int_text(value) for value in args):
            free_cells[(int(args[0]), int(args[1]))] = int(args[2])

    if width <= 0 or height <= 0:
        raise ValueError("Could not infer scene dimensions from active atoms")

    return SceneState(
        width=width,
        height=height,
        positions=positions,
        robot_dirs=robot_dirs,
        free_cells=free_cells,
        robots=sorted(robots),
        blocks=sorted(blocks),
    )


def load_frames(log_path: Path) -> tuple[list[Frame], set[tuple[int, int]]]:
    data = json.loads(log_path.read_text(encoding="utf-8"))
    initial_atoms = extract_initial_atoms(data)
    goal_cells = extract_goal_cells(data)
    frames: list[Frame] = []

    initial_state = build_state(initial_atoms)
    initial_keys = {atom_to_text(atom) for atom in initial_atoms}
    frames.append(
        Frame(
            index=0,
            label="Initial",
            answer="Initial base from formula.children[0].atoms_list",
            question=None,
            added_atoms=[],
            used_atoms=[],
            removed_atoms=[],
            active_atoms=initial_atoms,
            state=initial_state,
            added_entities=set(),
            used_entities=set(),
        )
    )

    previous_keys = initial_keys
    for offset, step in enumerate(data.get("log") or [], start=1):
        active_atoms = active_atoms_from_base(step.get("base") or [])
        active_keys = {atom_to_text(atom) for atom in active_atoms}
        removed_atoms = sorted(previous_keys - active_keys)
        previous_keys = active_keys
        added_atoms = step.get("atoms_added") or []
        used_atoms = step.get("atoms_used") or []
        frames.append(
            Frame(
                index=offset,
                label=f"Step {step.get('step', offset - 1)}",
                answer=str(step.get("answer", "")),
                question=step.get("question"),
                added_atoms=added_atoms,
                used_atoms=used_atoms,
                removed_atoms=removed_atoms,
                active_atoms=active_atoms,
                state=build_state(active_atoms),
                added_entities=infer_entities(added_atoms),
                used_entities=infer_entities(used_atoms),
            )
        )

    return frames, goal_cells


class SceneRenderer:
    def __init__(self, goal_cells: set[tuple[int, int]]) -> None:
        self.goal_cells = goal_cells
        self.title_font = self._load_font(28, bold=True)
        self.text_font = self._load_font(18)
        self.small_font = self._load_font(16)
        self.mono_font = self._load_font(15, mono=True)

    @staticmethod
    def _load_font(size: int, bold: bool = False, mono: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
        candidates: list[str]
        if mono:
            candidates = ["consola.ttf", "cour.ttf", "DejaVuSansMono.ttf"]
        elif bold:
            candidates = ["arialbd.ttf", "DejaVuSans-Bold.ttf"]
        else:
            candidates = ["arial.ttf", "DejaVuSans.ttf"]
        for candidate in candidates:
            try:
                return ImageFont.truetype(candidate, size=size)
            except OSError:
                continue
        return ImageFont.load_default()

    def render(self, frame: Frame) -> Image.Image:
        scene_width = MARGIN_LEFT + frame.state.width * CELL_SIZE + MARGIN_RIGHT
        scene_height = MARGIN_TOP + frame.state.height * CELL_SIZE + MARGIN_BOTTOM
        total_width = scene_width + PANEL_GAP + PANEL_WIDTH
        total_height = max(scene_height, 860)

        image = Image.new("RGBA", (total_width, total_height), "#eef2f7")
        draw = ImageDraw.Draw(image)

        self._draw_scene(draw, frame, scene_width, scene_height)
        self._draw_panel(draw, frame, scene_width + PANEL_GAP, total_height)
        return image

    def _draw_scene(self, draw: ImageDraw.ImageDraw, frame: Frame, scene_width: int, scene_height: int) -> None:
        draw.rounded_rectangle((18, 18, scene_width - 18, scene_height - 18), radius=20, fill=PANEL_BG, outline=PANEL_BORDER, width=2)
        draw.text((MARGIN_LEFT, 20), "Block Move Planning", fill=TEXT, font=self.title_font)
        subtitle = f"{frame.label}   q={frame.question if frame.question is not None else '-'}"
        draw.text((MARGIN_LEFT, 54), subtitle, fill=MUTED, font=self.small_font)

        for x in range(1, frame.state.width + 1):
            for y in range(1, frame.state.height + 1):
                cell = self._cell_box(x, y, frame.state.height)
                free_value = frame.state.free_cells.get((x, y), 0)
                fill = FREE_FILL if free_value == 1 else OCCUPIED_FILL
                draw.rectangle(cell, fill=fill, outline=GRID_LINE, width=1)

        for x, y in self.goal_cells:
            if 1 <= x <= frame.state.width and 1 <= y <= frame.state.height:
                draw.rounded_rectangle(self._cell_box(x, y, frame.state.height), radius=8, fill=TARGET_FILL, outline=TARGET_OUTLINE, width=2)

        for x in range(1, frame.state.width + 1):
            cell = self._cell_box(x, 1, frame.state.height)
            draw.text((cell[0] + CELL_SIZE / 2 - 5, scene_height - MARGIN_BOTTOM + 12), str(x), fill=MUTED, font=self.small_font)
        for y in range(1, frame.state.height + 1):
            cell = self._cell_box(1, y, frame.state.height)
            draw.text((20, cell[1] + CELL_SIZE / 2 - 9), str(y), fill=MUTED, font=self.small_font)

        draw.text((scene_width / 2 - 10, scene_height - 34), "X", fill=MUTED, font=self.small_font)
        draw.text((24, 86), "Y", fill=MUTED, font=self.small_font)

        for block in frame.state.blocks:
            if block in frame.state.positions:
                self._draw_block(draw, block, frame.state.positions[block], frame)
        for robot in frame.state.robots:
            if robot in frame.state.positions:
                self._draw_robot(draw, robot, frame.state.positions[robot], frame)

    def _draw_panel(self, draw: ImageDraw.ImageDraw, frame: Frame, panel_left: int, total_height: int) -> None:
        panel_right = panel_left + PANEL_WIDTH
        draw.rounded_rectangle((panel_left, 18, panel_right, total_height - 18), radius=20, fill=PANEL_BG, outline=PANEL_BORDER, width=2)

        y = 28
        y = self._draw_wrapped(draw, panel_left + 20, y, panel_right - 20, f"{frame.label}", self.title_font, TEXT)
        if frame.answer:
            y = self._draw_wrapped(draw, panel_left + 20, y + 6, panel_right - 20, f"Answer: {frame.answer}", self.small_font, MUTED)

        panel_items = [
            ("Added atoms", [atom_to_text(atom) for atom in frame.added_atoms], ADDED_HIGHLIGHT),
            ("Used atoms", [atom_to_text(atom) for atom in frame.used_atoms], USED_HIGHLIGHT),
            ("Removed from active base", frame.removed_atoms, "#dc2626"),
            ("Scene summary", self._scene_summary(frame.state), TEXT),
        ]

        for title, items, color in panel_items:
            y += 20
            draw.text((panel_left + 20, y), title, fill=color, font=self.text_font)
            y += 30
            if not items:
                draw.text((panel_left + 20, y), "-", fill=MUTED, font=self.small_font)
                y += LINE_HEIGHT
                continue
            for item in items:
                y = self._draw_bullet(draw, panel_left + 20, y, panel_right - 24, item)
                if y > total_height - 60:
                    draw.text((panel_left + 20, y), "...", fill=MUTED, font=self.small_font)
                    return

        legend_y = total_height - 150
        draw.text((panel_left + 20, legend_y), "Legend", fill=TEXT, font=self.text_font)
        legend_y += 28
        for label, color in [
            ("goal area", TARGET_OUTLINE),
            ("used entity", USED_HIGHLIGHT),
            ("added entity", ADDED_HIGHLIGHT),
            ("used + added", BOTH_HIGHLIGHT),
        ]:
            draw.rounded_rectangle((panel_left + 20, legend_y + 3, panel_left + 40, legend_y + 19), radius=4, fill=color, outline=color)
            draw.text((panel_left + 52, legend_y), label, fill=MUTED, font=self.small_font)
            legend_y += 24

    def _scene_summary(self, state: SceneState) -> list[str]:
        robots = []
        for robot in state.robots:
            pos = state.positions.get(robot)
            direction = state.robot_dirs.get(robot, "?")
            if pos:
                robots.append(f"{robot} @ ({pos[0]}, {pos[1]}) dir={direction}")
        blocks = []
        for block in state.blocks:
            pos = state.positions.get(block)
            if pos:
                blocks.append(f"{block} @ ({pos[0]}, {pos[1]}) cells=({pos[0]}, {pos[1]})-({pos[0] + 1}, {pos[1]})")
        return robots + blocks

    def _draw_bullet(self, draw: ImageDraw.ImageDraw, x: int, y: int, max_x: int, text: str) -> int:
        draw.text((x, y), "- ", fill=TEXT, font=self.small_font)
        return self._draw_wrapped(draw, x + 18, y, max_x, text, self.mono_font, TEXT)

    def _draw_wrapped(
        self,
        draw: ImageDraw.ImageDraw,
        x: int,
        y: int,
        max_x: int,
        text: str,
        font: ImageFont.FreeTypeFont | ImageFont.ImageFont,
        color: str,
    ) -> int:
        words = text.split()
        if not words:
            draw.text((x, y), "", fill=color, font=font)
            return y + LINE_HEIGHT

        line = words[0]
        for word in words[1:]:
            candidate = f"{line} {word}"
            if draw.textlength(candidate, font=font) <= max_x - x:
                line = candidate
            else:
                draw.text((x, y), line, fill=color, font=font)
                y += LINE_HEIGHT
                line = word
        draw.text((x, y), line, fill=color, font=font)
        return y + LINE_HEIGHT

    def _cell_box(self, x: int, y: int, height: int) -> tuple[int, int, int, int]:
        left = MARGIN_LEFT + (x - 1) * CELL_SIZE
        top = MARGIN_TOP + (height - y) * CELL_SIZE
        return (left, top, left + CELL_SIZE, top + CELL_SIZE)

    def _entity_highlight(self, entity: str, frame: Frame) -> str | None:
        is_used = entity in frame.used_entities
        is_added = entity in frame.added_entities
        if is_used and is_added:
            return BOTH_HIGHLIGHT
        if is_added:
            return ADDED_HIGHLIGHT
        if is_used:
            return USED_HIGHLIGHT
        return None

    def _draw_robot(self, draw: ImageDraw.ImageDraw, robot: str, position: tuple[int, int], frame: Frame) -> None:
        x, y = position
        box = self._cell_box(x, y, frame.state.height)
        cx = (box[0] + box[2]) / 2
        cy = (box[1] + box[3]) / 2
        radius = CELL_SIZE * 0.33

        highlight = self._entity_highlight(robot, frame)
        if highlight:
            draw.ellipse((cx - radius - 6, cy - radius - 6, cx + radius + 6, cy + radius + 6), outline=highlight, width=5)

        draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=ROBOT_FILL, outline="#1d4ed8", width=2)
        self._draw_arrow(draw, (cx, cy), frame.state.robot_dirs.get(robot, "?"))
        label = robot
        text_width = draw.textlength(label, font=self.small_font)
        draw.text((cx - text_width / 2, cy + radius + 8), label, fill=TEXT, font=self.small_font)

    def _draw_arrow(self, draw: ImageDraw.ImageDraw, center: tuple[float, float], direction: str) -> None:
        cx, cy = center
        shaft = 14
        head = 7
        if direction == "N":
            points = [(cx, cy - shaft), (cx - head, cy), (cx - 2, cy), (cx - 2, cy + shaft), (cx + 2, cy + shaft), (cx + 2, cy), (cx + head, cy)]
        elif direction == "E":
            points = [(cx + shaft, cy), (cx, cy - head), (cx, cy - 2), (cx - shaft, cy - 2), (cx - shaft, cy + 2), (cx, cy + 2), (cx, cy + head)]
        elif direction == "S":
            points = [(cx, cy + shaft), (cx - head, cy), (cx - 2, cy), (cx - 2, cy - shaft), (cx + 2, cy - shaft), (cx + 2, cy), (cx + head, cy)]
        elif direction == "W":
            points = [(cx - shaft, cy), (cx, cy - head), (cx, cy - 2), (cx + shaft, cy - 2), (cx + shaft, cy + 2), (cx, cy + 2), (cx, cy + head)]
        else:
            draw.text((cx - 6, cy - 9), "?", fill=ROBOT_TEXT, font=self.text_font)
            return
        draw.polygon(points, fill=ROBOT_TEXT)

    def _draw_block(self, draw: ImageDraw.ImageDraw, block: str, position: tuple[int, int], frame: Frame) -> None:
        x, y = position
        left_box = self._cell_box(x, y, frame.state.height)
        right_box = self._cell_box(x + 1, y, frame.state.height)
        block_box = (left_box[0] + 3, left_box[1] + 8, right_box[2] - 3, right_box[3] - 8)

        highlight = self._entity_highlight(block, frame)
        if highlight:
            draw.rounded_rectangle((block_box[0] - 6, block_box[1] - 6, block_box[2] + 6, block_box[3] + 6), radius=14, outline=highlight, width=5)

        draw.rounded_rectangle(block_box, radius=12, fill=BLOCK_FILL, outline="#b45309", width=2)
        label = block
        text_width = draw.textlength(label, font=self.text_font)
        text_x = (block_box[0] + block_box[2]) / 2 - text_width / 2
        text_y = (block_box[1] + block_box[3]) / 2 - 10
        draw.text((text_x, text_y), label, fill=BLOCK_TEXT, font=self.text_font)


class ViewerApp:
    def __init__(self, root: tk.Tk, frames: list[Frame], renderer: SceneRenderer, export_dir: Path) -> None:
        self.root = root
        self.frames = frames
        self.renderer = renderer
        self.export_dir = export_dir
        self.current_index = 0
        self.photo: ImageTk.PhotoImage | None = None

        self.root.title("Block Move Planning Viewer")
        self.root.geometry("1600x980")
        self.root.minsize(1200, 760)

        outer = ttk.Frame(root, padding=10)
        outer.pack(fill=tk.BOTH, expand=True)

        controls = ttk.Frame(outer)
        controls.pack(fill=tk.X)

        ttk.Button(controls, text="<< Prev", command=self.prev_frame).pack(side=tk.LEFT)
        ttk.Button(controls, text="Next >>", command=self.next_frame).pack(side=tk.LEFT, padx=(8, 0))
        ttk.Button(controls, text="Save PNG", command=self.save_current_frame).pack(side=tk.LEFT, padx=(16, 0))
        ttk.Button(controls, text="Open JSON", command=self.choose_and_reload).pack(side=tk.LEFT, padx=(8, 0))

        self.status_var = tk.StringVar()
        ttk.Label(controls, textvariable=self.status_var).pack(side=tk.RIGHT)

        self.canvas = tk.Canvas(outer, background="#eef2f7", highlightthickness=0)
        self.canvas.pack(fill=tk.BOTH, expand=True, pady=(10, 0))

        self.root.bind("<Left>", lambda _event: self.prev_frame())
        self.root.bind("<Right>", lambda _event: self.next_frame())
        self.root.bind("<Control-s>", lambda _event: self.save_current_frame())

        self.refresh()

    def choose_and_reload(self) -> None:
        selected = filedialog.askopenfilename(
            title="Select log JSON",
            filetypes=[("JSON files", "*.json"), ("All files", "*.*")],
        )
        if not selected:
            return
        try:
            frames, goal_cells = load_frames(Path(selected))
        except Exception as exc:  # pragma: no cover - GUI flow
            messagebox.showerror("Load error", str(exc))
            return

        self.frames = frames
        self.renderer = SceneRenderer(goal_cells)
        self.current_index = 0
        self.refresh()

    def current_frame(self) -> Frame:
        return self.frames[self.current_index]

    def refresh(self) -> None:
        frame = self.current_frame()
        image = self.renderer.render(frame).convert("RGB")
        self.photo = ImageTk.PhotoImage(image)
        self.canvas.delete("all")
        self.canvas.config(scrollregion=(0, 0, image.width, image.height))
        self.canvas.create_image(0, 0, anchor=tk.NW, image=self.photo)
        self.status_var.set(f"{self.current_index + 1}/{len(self.frames)}   {frame.label}")

    def prev_frame(self) -> None:
        if self.current_index > 0:
            self.current_index -= 1
            self.refresh()

    def next_frame(self) -> None:
        if self.current_index + 1 < len(self.frames):
            self.current_index += 1
            self.refresh()

    def save_current_frame(self) -> None:
        frame = self.current_frame()
        self.export_dir.mkdir(parents=True, exist_ok=True)
        default_name = f"{frame.label.lower().replace(' ', '_')}.png"
        selected = filedialog.asksaveasfilename(
            title="Save current frame",
            initialdir=str(self.export_dir),
            initialfile=default_name,
            defaultextension=".png",
            filetypes=[("PNG image", "*.png")],
        )
        if not selected:
            return
        image = self.renderer.render(frame).convert("RGB")
        image.save(selected, format="PNG")
        messagebox.showinfo("Saved", f"Saved to:\n{selected}")


def default_paths() -> tuple[Path, Path]:
    repo_root = Path(__file__).resolve().parents[2]
    json_path = repo_root / "problems" / "BlockMovePlanning.json"
    export_dir = Path(__file__).resolve().parent / "exports"
    return json_path, export_dir


def parse_args() -> argparse.Namespace:
    json_path, export_dir = default_paths()
    parser = argparse.ArgumentParser(description="Viewer for BlockMovePlanning solver log JSON")
    parser.add_argument("--json", type=Path, default=json_path, help="Path to solver log JSON")
    parser.add_argument("--export-dir", type=Path, default=export_dir, help="Default directory for exported PNG files")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    frames, goal_cells = load_frames(args.json)
    root = tk.Tk()
    style = ttk.Style(root)
    if "vista" in style.theme_names():
        style.theme_use("vista")
    app = ViewerApp(root, frames, SceneRenderer(goal_cells), args.export_dir)
    app.refresh()
    root.mainloop()


if __name__ == "__main__":
    main()
