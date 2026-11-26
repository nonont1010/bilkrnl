"""TRON-neon techno corn-kernel brand marks for bilkrnl.

Self-contained SVG (no webfonts — GitHub camo / offline safe). Wordmark is a
geometric retro path font. Husk = shell pun for the UEFI/kernel stack.
"""
from __future__ import annotations

import shutil
import subprocess
from pathlib import Path
from xml.sax.saxutils import escape

# Palette — futuristic neon grid + corn gold
OBSIDIAN = "#050811"
SLATE = "#0b1326"
CYAN = "#00e5ff"
SKY = "#38bdf8"
PLASMA = "#0284c7"
AMBER = "#ffb800"
GOLD = "#ffd54a"
MAGENTA = "#ff2a85"
EMERALD = "#00ff9d"
ICE = "#f0f6fc"
MUTED = "#94a3b8"


# ---------------------------------------------------------------------------
# Geometric retro wordmark (7-seg-ish / TRON angular capitals)
# Each glyph: list of closed polygons in a 5×7 unit cell.
# ---------------------------------------------------------------------------
_GLYPHS: dict[str, list[list[tuple[float, float]]]] = {
    "b": [
        [(0, 0), (3.2, 0), (4.2, 1), (4.2, 2.6), (3.4, 3.2), (4.2, 3.8), (4.2, 6), (3.2, 7), (0, 7)],
        [(1.2, 1.1), (2.8, 1.1), (3.1, 1.5), (3.1, 2.7), (2.8, 3.1), (1.2, 3.1)],
        [(1.2, 3.9), (2.9, 3.9), (3.2, 4.3), (3.2, 5.7), (2.8, 6.0), (1.2, 6.0)],
    ],
    "i": [
        [(0.8, 0), (3.2, 0), (3.2, 1.1), (2.6, 1.1), (2.6, 5.9), (3.2, 5.9), (3.2, 7), (0.8, 7), (0.8, 5.9), (1.4, 5.9), (1.4, 1.1), (0.8, 1.1)],
    ],
    "l": [
        [(0.5, 0), (2.0, 0), (2.0, 5.7), (3.8, 5.7), (3.8, 7), (0.5, 7)],
    ],
    "k": [
        # stem
        [(0.3, 0), (1.5, 0), (1.5, 7), (0.3, 7)],
        # upper arm
        [(1.5, 3.2), (3.5, 0.2), (4.5, 0.2), (2.3, 3.5)],
        # lower arm
        [(1.5, 3.5), (2.3, 3.5), (4.5, 6.8), (3.5, 6.8)],
    ],
    "r": [
        [(0.3, 0), (1.5, 0), (1.5, 7), (0.3, 7)],
        [(1.5, 0), (3.3, 0), (4.2, 0.9), (4.2, 2.6), (3.3, 3.5), (1.5, 3.5)],
        [(1.5, 1.15), (3.0, 1.15), (3.15, 1.4), (3.15, 2.4), (3.0, 2.65), (1.5, 2.65)],
        [(2.2, 3.5), (3.3, 3.5), (4.5, 6.8), (3.3, 6.8)],
    ],
    "n": [
        [(0.3, 0), (1.5, 0), (1.5, 7), (0.3, 7)],
        [(3.3, 0), (4.5, 0), (4.5, 7), (3.3, 7)],
        [(1.5, 0.3), (2.7, 0.3), (3.3, 4.8), (3.3, 7), (2.1, 7), (1.5, 2.2)],
    ],
}


def _poly(points: list[tuple[float, float]], *, fill: str, opacity: float = 1.0) -> str:
    pts = " ".join(f"{x:.2f},{y:.2f}" for x, y in points)
    op = f' fill-opacity="{opacity}"' if opacity < 1.0 else ""
    return f'<polygon points="{pts}" fill="{fill}"{op}/>'


def wordmark_svg(
    text: str = "bilkrnl",
    *,
    x: float,
    y: float,
    cell_w: float = 22.0,
    cell_h: float = 30.0,
    tracking: float = 6.0,
    fill: str = CYAN,
    glow_id: str = "wmGlow",
) -> tuple[str, float]:
    """Return (svg fragment, total width)."""
    parts: list[str] = [f'<g transform="translate({x:.1f},{y:.1f})" filter="url(#{glow_id})">']
    cursor = 0.0
    for ch in text.lower():
        glyphs = _GLYPHS.get(ch)
        if not glyphs:
            cursor += cell_w * 0.4
            continue
        sx = cell_w / 5.0
        sy = cell_h / 7.0
        parts.append(f'<g transform="translate({cursor:.1f},0) scale({sx:.3f},{sy:.3f})">')
        for poly in glyphs:
            parts.append(_poly(poly, fill=fill))
        parts.append("</g>")
        cursor += cell_w + tracking
    parts.append("</g>")
    # underline scan bar
    bar_y = y + cell_h + 8
    parts.append(
        f'<rect x="{x:.1f}" y="{bar_y:.1f}" width="{cursor - tracking:.1f}" height="2.5" '
        f'fill="{AMBER}" filter="url(#{glow_id})" opacity="0.85"/>'
    )
    parts.append(
        f'<rect x="{x:.1f}" y="{bar_y + 5:.1f}" width="{(cursor - tracking) * 0.42:.1f}" height="1.5" '
        f'fill="{MAGENTA}" opacity="0.7"/>'
    )
    return "\n".join(parts), cursor - tracking


def _defs() -> str:
    return f"""
  <defs>
    <linearGradient id="bgGrad" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="{OBSIDIAN}"/>
      <stop offset="55%" stop-color="#070d1a"/>
      <stop offset="100%" stop-color="#0a1224"/>
    </linearGradient>
    <radialGradient id="glowOrb" cx="35%" cy="45%" r="45%">
      <stop offset="0%" stop-color="{CYAN}" stop-opacity="0.22"/>
      <stop offset="55%" stop-color="{PLASMA}" stop-opacity="0.06"/>
      <stop offset="100%" stop-color="{OBSIDIAN}" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="kernelGrad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="{GOLD}"/>
      <stop offset="45%" stop-color="{AMBER}"/>
      <stop offset="100%" stop-color="#c47a00"/>
    </linearGradient>
    <linearGradient id="huskGrad" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="{CYAN}"/>
      <stop offset="50%" stop-color="{SKY}"/>
      <stop offset="100%" stop-color="{MAGENTA}"/>
    </linearGradient>
    <linearGradient id="huskFill" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#062a33" stop-opacity="0.85"/>
      <stop offset="100%" stop-color="#1a0530" stop-opacity="0.55"/>
    </linearGradient>
    <filter id="neonGlow" x="-40%" y="-40%" width="180%" height="180%">
      <feGaussianBlur stdDeviation="3.5" result="b"/>
      <feMerge>
        <feMergeNode in="b"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
    <filter id="wmGlow" x="-30%" y="-40%" width="160%" height="180%">
      <feGaussianBlur stdDeviation="2.2" result="b"/>
      <feMerge>
        <feMergeNode in="b"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
    <filter id="softGlow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="6" result="b"/>
      <feMerge>
        <feMergeNode in="b"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
    <pattern id="cyberGrid" width="32" height="32" patternUnits="userSpaceOnUse">
      <path d="M 32 0 L 0 0 0 32" fill="none" stroke="{CYAN}" stroke-opacity="0.10" stroke-width="1"/>
    </pattern>
  </defs>
"""


def _grid_overlay(w: float, h: float) -> str:
    return (
        f'<rect width="{w}" height="{h}" fill="url(#cyberGrid)"/>'
        f'<circle cx="{w * 0.28:.0f}" cy="{h * 0.48:.0f}" r="{min(w, h) * 0.42:.0f}" fill="url(#glowOrb)"/>'
    )


def corn_emblem(*, cx: float, cy: float, scale: float = 1.0) -> str:
    """Techno corn cob nestled in neon husk/shell leaves."""
    s = scale
    parts: list[str] = [f'<g transform="translate({cx:.1f},{cy:.1f}) scale({s:.3f})" filter="url(#neonGlow)">']

    # Outer husk / shell — left leaf
    left_husk = [
        (-78, -10),
        (-95, -55),
        (-70, -110),
        (-35, -125),
        (-18, -95),
        (-28, -40),
        (-22, 20),
        (-35, 85),
        (-55, 120),
        (-85, 95),
        (-98, 40),
    ]
    # Right husk
    right_husk = [
        (78, -5),
        (98, -50),
        (72, -108),
        (38, -128),
        (20, -96),
        (30, -38),
        (24, 25),
        (40, 90),
        (68, 125),
        (95, 88),
        (105, 30),
    ]
    # Back husk tip (above cob)
    tip = [(-12, -118), (0, -148), (14, -118), (8, -100), (-6, -100)]

    parts.append(_poly(left_husk, fill="#062a33", opacity=0.92))
    parts.append(
        f'<polyline points="{" ".join(f"{x},{y}" for x, y in left_husk)}" '
        f'fill="none" stroke="url(#huskGrad)" stroke-width="3.2" stroke-linejoin="round"/>'
    )
    parts.append(_poly(right_husk, fill="#1a0530", opacity=0.75))
    parts.append(
        f'<polyline points="{" ".join(f"{x},{y}" for x, y in right_husk)}" '
        f'fill="none" stroke="url(#huskGrad)" stroke-width="3.2" stroke-linejoin="round"/>'
    )
    parts.append(_poly(tip, fill="#0a1a28", opacity=0.9))
    parts.append(
        f'<polyline points="{" ".join(f"{x},{y}" for x, y in tip)}" '
        f'fill="none" stroke="{CYAN}" stroke-width="2.4" stroke-linejoin="round"/>'
    )

    # Circuit etch lines on husks
    for path in (
        "M -88,-40 L -70,-70 L -50,-85",
        "M -92,20 L -70,55 L -55,80",
        "M 90,-35 L 68,-72 L 48,-90",
        "M 96,25 L 72,60 L 55,88",
    ):
        parts.append(
            f'<path d="{path}" fill="none" stroke="{EMERALD}" stroke-opacity="0.45" '
            f'stroke-width="1.2" stroke-linecap="round"/>'
        )

    # Cob body (ellipse)
    parts.append(
        f'<ellipse cx="0" cy="5" rx="42" ry="95" fill="url(#kernelGrad)" '
        f'stroke="{AMBER}" stroke-width="2.5"/>'
    )
    # Kernel lattice — techno rows
    for row, yy in enumerate(range(-70, 80, 16)):
        stagger = 7 if row % 2 else 0
        for xx in range(-28, 30, 14):
            kx = xx + stagger
            # hex-ish kernel
            kr = 5.5
            kernel = [
                (kx, yy - kr),
                (kx + kr * 0.85, yy - kr * 0.35),
                (kx + kr * 0.85, yy + kr * 0.35),
                (kx, yy + kr),
                (kx - kr * 0.85, yy + kr * 0.35),
                (kx - kr * 0.85, yy - kr * 0.35),
            ]
            # clip roughly to ellipse
            if (kx * kx) / (38 * 38) + (yy * yy) / (88 * 88) > 1.0:
                continue
            fill = GOLD if (row + xx) % 3 else AMBER
            parts.append(_poly(kernel, fill=fill, opacity=0.92))
            parts.append(
                f'<polygon points="{" ".join(f"{x:.1f},{y:.1f}" for x, y in kernel)}" '
                f'fill="none" stroke="{OBSIDIAN}" stroke-opacity="0.35" stroke-width="0.6"/>'
            )

    # Neon midline / "silk" circuit
    parts.append(
        f'<path d="M 0,-88 C 6,-40 6,40 0,92" fill="none" stroke="{CYAN}" '
        f'stroke-width="1.4" stroke-opacity="0.55"/>'
    )
    parts.append(
        f'<circle cx="0" cy="-96" r="4" fill="{MAGENTA}" stroke="{ICE}" stroke-width="1"/>'
    )
    # Capsule HUD brackets around cob
    parts.append(
        f'<path d="M -55,-100 L -62,-100 L -62,-70" fill="none" stroke="{CYAN}" '
        f'stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M 55,-100 L 62,-100 L 62,-70" fill="none" stroke="{CYAN}" '
        f'stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M -55,110 L -62,110 L -62,80" fill="none" stroke="{MAGENTA}" '
        f'stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M 55,110 L 62,110 L 62,80" fill="none" stroke="{MAGENTA}" '
        f'stroke-width="1.8" stroke-linecap="square"/>'
    )

    parts.append("</g>")
    return "\n".join(parts)


def _hud_frame(w: float, h: float, pad: float = 14.0) -> str:
    x0, y0 = pad, pad
    x1, y1 = w - pad, h - pad
    L = 28
    segs = [
        f"M {x0},{y0 + L} L {x0},{y0} L {x0 + L},{y0}",
        f"M {x1 - L},{y0} L {x1},{y0} L {x1},{y0 + L}",
        f"M {x0},{y1 - L} L {x0},{y1} L {x0 + L},{y1}",
        f"M {x1 - L},{y1} L {x1},{y1} L {x1},{y1 - L}",
    ]
    return "\n".join(
        f'<path d="{d}" fill="none" stroke="{CYAN}" stroke-width="2" stroke-opacity="0.75"/>'
        for d in segs
    )


def _svg_doc(w: int, h: int, body: str, *, title: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" role="img">
  <title>{escape(title)}</title>
{_defs()}
  <rect width="{w}" height="{h}" fill="url(#bgGrad)"/>
  {_grid_overlay(w, h)}
  {_hud_frame(w, h)}
{body}
</svg>
"""


def logo_banner_svg() -> str:
    """Wide hero: corn emblem + retro bilkrnl wordmark (~1200×360)."""
    w, h = 1200, 360
    emblem = corn_emblem(cx=210, cy=180, scale=1.15)
    wm, wm_w = wordmark_svg("bilkrnl", x=400, y=118, cell_w=58, cell_h=78, tracking=14, fill=CYAN)
    tag = (
        f'<text x="400" y="268" fill="{MUTED}" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" '
        f'font-size="16" letter-spacing="3">UEFI · SoftISA · BLAST CHAMBER</text>'
    )
    # small amber accent chips
    chips = (
        f'<g font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="12" fill="{AMBER}">'
        f'<rect x="400" y="288" width="52" height="20" rx="2" fill="none" stroke="{AMBER}" stroke-opacity="0.7"/>'
        f'<text x="410" y="302">1A</text>'
        f'<rect x="462" y="288" width="52" height="20" rx="2" fill="none" stroke="{EMERALD}" stroke-opacity="0.7"/>'
        f'<text x="472" y="302" fill="{EMERALD}">2C</text>'
        f'<rect x="524" y="288" width="72" height="20" rx="2" fill="none" stroke="{MUTED}" stroke-opacity="0.5"/>'
        f'<text x="534" y="302" fill="{MUTED}">1C…</text>'
        f"</g>"
    )
    # right-side vertical scan ticks
    ticks = "\n".join(
        f'<rect x="{1100 + (i % 3) * 8}" y="{60 + i * 22}" width="18" height="2" '
        f'fill="{CYAN}" opacity="{0.25 + (i % 4) * 0.12}"/>'
        for i in range(11)
    )
    body = f"{emblem}\n{wm}\n{tag}\n{chips}\n{ticks}"
    # ensure wm_w referenced (lint quiet) — width used for composition
    _ = wm_w
    return _svg_doc(w, h, body, title="bilkrnl — TRON neon techno corn-kernel")


def icon_svg() -> str:
    """Square emblem for favicon / root assets (512²)."""
    w = h = 512
    emblem = corn_emblem(cx=256, cy=230, scale=1.55)
    # compact wordmark under cob
    wm, _ = wordmark_svg("bilkrnl", x=86, y=420, cell_w=28, cell_h=36, tracking=6, fill=CYAN)
    body = f"{emblem}\n{wm}"
    return _svg_doc(w, h, body, title="bilkrnl icon")


def category_icon_svg(kind: str) -> str:
    """Small 256² category badges with husk + glyph accent."""
    w = h = 256
    emblem = corn_emblem(cx=128, cy=108, scale=0.72)
    labels = {
        "icon-isa": ("ISA", CYAN),
        "icon-enclave": ("ENC", EMERALD),
        "icon-chamber": ("BLST", AMBER),
        "icon-uefi": ("UEFI", MAGENTA),
    }
    label, color = labels.get(kind, ("BIL", CYAN))
    tag = (
        f'<text x="128" y="230" text-anchor="middle" fill="{color}" '
        f'font-family="ui-monospace, SFMono-Regular, Menlo, monospace" '
        f'font-size="22" font-weight="700" letter-spacing="4">{escape(label)}</text>'
    )
    return _svg_doc(w, h, f"{emblem}\n{tag}", title=f"bilkrnl {label}")


def _svg_to_png(svg_path: Path, png_path: Path, *, density: int = 144) -> None:
    """Rasterize via ImageMagick (available on host)."""
    magick = shutil.which("magick") or shutil.which("convert")
    if not magick:
        raise SystemExit("ImageMagick (magick/convert) required to rasterize brand SVGs")
    subprocess.check_call(
        [
            magick,
            "-background",
            "none",
            "-density",
            str(density),
            str(svg_path),
            str(png_path),
        ],
    )


def write_pair(svg_text: str, out_dir: Path, stem: str, *, density: int = 144) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    svg_path = out_dir / f"{stem}.svg"
    png_path = out_dir / f"{stem}.png"
    svg_path.write_text(svg_text, encoding="utf-8")
    _svg_to_png(svg_path, png_path, density=density)


def render_brand(assets: Path, root_assets: Path, icons: Path) -> None:
    """Emit logo banner, square icon, root copy, and category badges."""
    write_pair(logo_banner_svg(), assets, "logo", density=144)
    write_pair(icon_svg(), assets, "icon", density=144)
    # root assets mirror the square mark
    write_pair(icon_svg(), root_assets, "logo", density=144)
    for name in ("icon-isa", "icon-enclave", "icon-chamber", "icon-uefi"):
        write_pair(category_icon_svg(name), icons, name, density=128)


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[2]
    render_brand(root / "docs" / "assets", root / "assets", root / "docs" / "assets" / "icons")
    print("brand_tron_corn: wrote logo/icon/category assets")
