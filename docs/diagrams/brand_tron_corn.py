"""TRON-neon techno corn-kernel brand marks for bilkrnl.

Self-contained SVG (no webfonts — GitHub camo / offline safe). Wordmark is a
geometric retro path font. Husk = isolation shell; cob = SoftISA kernel;
banner rail encodes isolate → step → dispose → ratify (architecture, not roadmap).
"""
from __future__ import annotations

import math
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


def corn_emblem(*, cx: float, cy: float, scale: float = 1.0, chamber_ring: bool = False) -> str:
    """Stocky techno corn cob in neon husk/shell (isolation). Optional chamber hex ring."""
    s = scale
    parts: list[str] = [f'<g transform="translate({cx:.1f},{cy:.1f}) scale({s:.3f})" filter="url(#neonGlow)">']

    # Stocky cob: ~1:1.2 aspect (was ~1:2.26 cigar). Local units before scale.
    rx, ry = 60.0, 72.0

    if chamber_ring:
        # Regular hexagon — blast-chamber seal around the enclave
        R = 118.0
        hex_pts = []
        for i in range(6):
            ang = math.radians(-90 + i * 60)
            hex_pts.append((R * math.cos(ang), R * math.sin(ang)))
        pts = " ".join(f"{x:.1f},{y:.1f}" for x, y in hex_pts)
        parts.append(
            f'<polygon points="{pts}" fill="none" stroke="{CYAN}" stroke-width="5" '
            f'stroke-opacity="0.88" filter="url(#softGlow)"/>'
        )
        parts.append(
            f'<polygon points="{pts}" fill="none" stroke="{MAGENTA}" stroke-width="1.4" '
            f'stroke-opacity="0.45" stroke-dasharray="6 8"/>'
        )

    # Husk leaves — flare from mid-cob, narrower at top so cob "peels out"
    left_husk = [
        (-rx * 0.35, ry * 0.15),
        (-rx - 18, -ry * 0.35),
        (-rx - 8, -ry * 0.85),
        (-rx * 0.45, -ry * 0.95),
        (-rx * 0.25, -ry * 0.55),
        (-rx * 0.55, ry * 0.05),
        (-rx * 0.4, ry * 0.55),
        (-rx - 22, ry * 0.92),
        (-rx - 38, ry * 0.55),
        (-rx - 28, ry * 0.05),
    ]
    right_husk = [( -x, y) for x, y in left_husk]
    tip = [
        (-10, -ry * 0.92),
        (0, -ry - 22),
        (10, -ry * 0.92),
        (6, -ry * 0.72),
        (-6, -ry * 0.72),
    ]

    parts.append(_poly(left_husk, fill="#062a33", opacity=0.9))
    parts.append(
        f'<polyline points="{" ".join(f"{x:.1f},{y:.1f}" for x, y in left_husk)}" '
        f'fill="none" stroke="url(#huskGrad)" stroke-width="3" stroke-linejoin="round"/>'
    )
    parts.append(_poly(right_husk, fill="#1a0530", opacity=0.72))
    parts.append(
        f'<polyline points="{" ".join(f"{x:.1f},{y:.1f}" for x, y in right_husk)}" '
        f'fill="none" stroke="url(#huskGrad)" stroke-width="3" stroke-linejoin="round"/>'
    )
    parts.append(_poly(tip, fill="#0a1a28", opacity=0.88))
    parts.append(
        f'<polyline points="{" ".join(f"{x:.1f},{y:.1f}" for x, y in tip)}" '
        f'fill="none" stroke="{CYAN}" stroke-width="2.2" stroke-linejoin="round"/>'
    )

    for path in (
        f"M {-rx - 20},{-ry * 0.2} L {-rx - 6},{-ry * 0.55} L {-rx * 0.55},{-ry * 0.7}",
        f"M {-rx - 24},{ry * 0.25} L {-rx - 8},{ry * 0.55} L {-rx * 0.5},{ry * 0.75}",
        f"M {rx + 20},{-ry * 0.2} L {rx + 6},{-ry * 0.55} L {rx * 0.55},{-ry * 0.7}",
        f"M {rx + 24},{ry * 0.25} L {rx + 8},{ry * 0.55} L {rx * 0.5},{ry * 0.75}",
    ):
        parts.append(
            f'<path d="{path}" fill="none" stroke="{EMERALD}" stroke-opacity="0.5" '
            f'stroke-width="1.3" stroke-linecap="round"/>'
        )

    # Cob body + soft inner highlight
    parts.append(
        f'<ellipse cx="0" cy="2" rx="{rx}" ry="{ry}" fill="url(#kernelGrad)" '
        f'stroke="{AMBER}" stroke-width="3.5"/>'
    )
    parts.append(
        f'<ellipse cx="-6" cy="-8" rx="{rx * 0.55:.1f}" ry="{ry * 0.62:.1f}" '
        f'fill="{ICE}" fill-opacity="0.12"/>'
    )

    # Kernel lattice — 5×7 hex cells, stocky cob clip
    cell, gap = 11.0, 3.5
    cols, rows = 5, 7
    grid_w = cols * cell + (cols - 1) * gap
    grid_h = rows * cell + (rows - 1) * gap
    x0 = -grid_w / 2 + cell / 2
    y0 = -grid_h / 2 + cell / 2 - 2
    for row in range(rows):
        stagger = (cell + gap) * 0.5 if row % 2 else 0.0
        for col in range(cols):
            kx = x0 + col * (cell + gap) + stagger
            yy = y0 + row * (cell + gap)
            if (kx * kx) / ((rx - 8) ** 2) + (yy * yy) / ((ry - 8) ** 2) > 1.0:
                continue
            kr = cell * 0.42
            kernel = [
                (kx, yy - kr),
                (kx + kr * 0.85, yy - kr * 0.35),
                (kx + kr * 0.85, yy + kr * 0.35),
                (kx, yy + kr),
                (kx - kr * 0.85, yy + kr * 0.35),
                (kx - kr * 0.85, yy - kr * 0.35),
            ]
            # Magenta "ratified" kernel near top-center; else gold/amber lattice
            if row == 1 and col == 2:
                fill = MAGENTA
            elif col == 2:
                fill = GOLD if row % 2 else AMBER
            else:
                fill = AMBER if (row + col) % 2 else GOLD
            parts.append(_poly(kernel, fill=fill, opacity=0.95))
            parts.append(
                f'<polygon points="{" ".join(f"{x:.1f},{y:.1f}" for x, y in kernel)}" '
                f'fill="none" stroke="{OBSIDIAN}" stroke-opacity="0.4" stroke-width="0.7"/>'
            )

    # Silk / SoftISA step path through cob
    parts.append(
        f'<path d="M 0,{-ry + 6:.1f} C 5,{-ry * 0.3:.1f} 5,{ry * 0.35:.1f} 0,{ry - 6:.1f}" '
        f'fill="none" stroke="{CYAN}" stroke-width="1.6" stroke-opacity="0.65"/>'
    )
    parts.append(
        f'<circle cx="0" cy="{-ry + 2:.1f}" r="3.5" fill="{MAGENTA}" stroke="{ICE}" stroke-width="1"/>'
    )

    # Compact HUD brackets (match stocky height)
    by = ry + 8
    ty = -ry - 4
    parts.append(
        f'<path d="M {-rx - 8},{ty} L {-rx - 16},{ty} L {-rx - 16},{ty + 22}" fill="none" '
        f'stroke="{CYAN}" stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M {rx + 8},{ty} L {rx + 16},{ty} L {rx + 16},{ty + 22}" fill="none" '
        f'stroke="{CYAN}" stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M {-rx - 8},{by} L {-rx - 16},{by} L {-rx - 16},{by - 22}" fill="none" '
        f'stroke="{MAGENTA}" stroke-width="1.8" stroke-linecap="square"/>'
    )
    parts.append(
        f'<path d="M {rx + 8},{by} L {rx + 16},{by} L {rx + 16},{by - 22}" fill="none" '
        f'stroke="{MAGENTA}" stroke-width="1.8" stroke-linecap="square"/>'
    )

    parts.append("</g>")
    return "\n".join(parts)


def architecture_rail(*, x: float, y: float) -> str:
    """Banner secondary: isolate → step → dispose → ratify (+ weaken-only ratchet)."""
    mono = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
    nodes = [
        (0, "ISOLATE", CYAN, "hex"),
        (168, "STEP", AMBER, "tick"),
        (336, "DISPOSE", MAGENTA, "burst"),
        (504, "RATIFY", EMERALD, "seal"),
    ]
    parts = [f'<g transform="translate({x:.1f},{y:.1f})" font-family="{mono}">']
    # rail line
    parts.append(
        f'<line x1="14" y1="0" x2="518" y2="0" stroke="{MUTED}" stroke-opacity="0.35" stroke-width="1.5"/>'
    )
    for nx, label, color, kind in nodes:
        if kind == "hex":
            parts.append(
                f'<polygon points="{nx},-10 {nx + 9},-5 {nx + 9},5 {nx},10 {nx - 9},5 {nx - 9},-5" '
                f'fill="none" stroke="{color}" stroke-width="2"/>'
            )
        elif kind == "tick":
            parts.append(f'<circle cx="{nx}" cy="0" r="5" fill="{color}"/>')
            parts.append(
                f'<path d="M {nx - 14},-1 L {nx - 6},-1" stroke="{color}" stroke-width="2" '
                f'stroke-linecap="round"/>'
            )
        elif kind == "burst":
            parts.append(f'<circle cx="{nx}" cy="0" r="4" fill="{color}"/>')
            for ang in (0, 60, 120, 180, 240, 300):
                rad = math.radians(ang)
                x2, y2 = nx + 11 * math.cos(rad), 11 * math.sin(rad)
                parts.append(
                    f'<line x1="{nx}" y1="0" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" '
                    f'stroke-width="1.4" stroke-opacity="0.85"/>'
                )
        else:  # seal
            parts.append(
                f'<circle cx="{nx}" cy="0" r="9" fill="none" stroke="{color}" stroke-width="2"/>'
            )
            parts.append(
                f'<path d="M {nx - 4},0 L {nx - 1},4 L {nx + 5},-4" fill="none" stroke="{color}" '
                f'stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>'
            )
        parts.append(
            f'<text x="{nx}" y="26" text-anchor="middle" fill="{color}" font-size="11" '
            f'letter-spacing="1.5">{label}</text>'
        )
    # capability ratchet — weaken-only descending bars
    bx = 560
    parts.append(
        f'<text x="{bx}" y="-8" fill="{MUTED}" font-size="10" letter-spacing="1">CAP↓</text>'
    )
    for i, hbar in enumerate((14, 10, 6)):
        parts.append(
            f'<rect x="{bx + i * 16}" y="{8 - hbar}" width="12" height="{hbar}" rx="1" '
            f'fill="{AMBER}" fill-opacity="{0.95 - i * 0.25}"/>'
        )
    parts.append(
        f'<text x="{bx}" y="28" fill="{MUTED}" font-size="9" letter-spacing="0.5">weaken-only</text>'
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
    """Wide hero: stocky corn + wordmark + architecture rail (~1200×360)."""
    w, h = 1200, 360
    emblem = corn_emblem(cx=200, cy=168, scale=1.05, chamber_ring=False)
    # Wider, shorter letter cells — less vertical stretch than cell_h=78
    wm, wm_w = wordmark_svg(
        "bilkrnl", x=380, y=88, cell_w=62, cell_h=52, tracking=12, fill=CYAN
    )
    tag = (
        f'<text x="382" y="178" fill="{MUTED}" '
        f'font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" '
        f'font-size="15" letter-spacing="2.5">'
        f"Isolate. Step. Dispose. Ratify. Trust nothing.</text>"
    )
    rail = architecture_rail(x=382, y=230)
    ticks = "\n".join(
        f'<rect x="{1100 + (i % 3) * 8}" y="{48 + i * 24}" width="18" height="2" '
        f'fill="{CYAN}" opacity="{0.25 + (i % 4) * 0.12}"/>'
        for i in range(11)
    )
    _ = wm_w
    body = f"{emblem}\n{wm}\n{tag}\n{rail}\n{ticks}"
    return _svg_doc(w, h, body, title="bilkrnl — isolate · step · dispose · ratify")


def icon_svg() -> str:
    """Square chamber-sealed cob for favicon / root assets (512²). No wordmark chips."""
    w = h = 512
    emblem = corn_emblem(cx=256, cy=256, scale=1.85, chamber_ring=True)
    return _svg_doc(w, h, emblem, title="bilkrnl icon")


def category_icon_svg(kind: str) -> str:
    """Small 256² category badges with husk + glyph accent."""
    w = h = 256
    emblem = corn_emblem(cx=128, cy=112, scale=0.95)
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
