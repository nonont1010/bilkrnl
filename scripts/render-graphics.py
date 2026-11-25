#!/usr/bin/env python3
"""Deterministic neon-grid SVG/PNG kit for bilkrnl docs.

Emits brand assets and architecture/workflow showcase graphics under docs/assets/
and assets/. Re-run at the end of scripts/verify.sh (build stage refresh).
"""
from __future__ import annotations

import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS_ASSETS = ROOT / "docs" / "assets"
EXAMPLES = DOCS_ASSETS / "examples"
ICONS = DOCS_ASSETS / "icons"
ROOT_ASSETS = ROOT / "assets"

FONT_UI = "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif"
FONT_MONO = "ui-monospace, 'Cascadia Code', 'Fira Code', Menlo, Consolas, monospace"


def defs_block() -> str:
    return """
  <defs>
    <linearGradient id="bg-grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#04070f" />
      <stop offset="50%" stop-color="#081022" />
      <stop offset="100%" stop-color="#040814" />
    </linearGradient>
    <radialGradient id="core-glow" cx="50%" cy="45%" r="55%">
      <stop offset="0%" stop-color="#00e5ff" stop-opacity="0.16" />
      <stop offset="60%" stop-color="#0284c7" stop-opacity="0.05" />
      <stop offset="100%" stop-color="#050811" stop-opacity="0" />
    </radialGradient>
    <pattern id="minor-grid" width="24" height="24" patternUnits="userSpaceOnUse">
      <path d="M 24 0 L 0 0 0 24" fill="none" stroke="#38bdf8" stroke-width="0.5" stroke-opacity="0.08" />
    </pattern>
    <pattern id="major-grid" width="96" height="96" patternUnits="userSpaceOnUse">
      <rect width="96" height="96" fill="url(#minor-grid)" />
      <path d="M 96 0 L 0 0 0 96" fill="none" stroke="#00e5ff" stroke-width="0.9" stroke-opacity="0.18" />
      <circle cx="0" cy="0" r="1.5" fill="#00e5ff" fill-opacity="0.35" />
    </pattern>
    <filter id="neon-glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="3.5" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>
"""


def hud_reticles(x: int, y: int, w: int, h: int, size: int = 12, color: str = "#00e5ff") -> str:
    return "\n".join(
        [
            f'<path d="M {x} {y + size} L {x} {y} L {x + size} {y}" fill="none" stroke="{color}" stroke-width="2"/>',
            f'<path d="M {x + w - size} {y} L {x + w} {y} L {x + w} {y + size}" fill="none" stroke="{color}" stroke-width="2"/>',
            f'<path d="M {x} {y + h - size} L {x} {y + h} L {x + size} {y + h}" fill="none" stroke="{color}" stroke-width="2"/>',
            f'<path d="M {x + w - size} {y + h} L {x + w} {y + h} L {x + w} {y + h - size}" fill="none" stroke="{color}" stroke-width="2"/>',
        ]
    )


def canvas(w: int, h: int, body: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" height="auto" role="img">
{defs_block()}
  <rect width="{w}" height="{h}" fill="url(#bg-grad)" />
  <rect width="{w}" height="{h}" fill="url(#major-grid)" />
  <rect width="{w}" height="{h}" fill="url(#core-glow)" />
{body}
</svg>
"""


def panel(x: int, y: int, w: int, h: int, title: str = "") -> str:
    parts = [
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="#0b1428" fill-opacity="0.92" stroke="#00e5ff" stroke-width="1.5" filter="url(#neon-glow)"/>',
        hud_reticles(x, y, w, h),
    ]
    if title:
        parts.append(
            f'<text x="{x + 16}" y="{y + 28}" fill="#f0f6fc" font-family="{FONT_UI}" font-size="16" font-weight="700">{title}</text>'
        )
    return "\n".join(parts)


def write_svg(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    ET.parse(path)  # well-formedness gate


def rasterize(svg: Path, png: Path, dpi: int = 192) -> None:
    png.parent.mkdir(parents=True, exist_ok=True)
    # Prefer inkscape, then magick
    if subprocess.call(["inkscape", str(svg), f"--export-filename={png}", f"--export-dpi={dpi}"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) == 0:
        pass
    else:
        subprocess.check_call(
            ["magick", "-density", str(dpi), str(svg), str(png)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    data = png.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise SystemExit(f"bad PNG magic: {png}")


def emblem(cx: int, cy: int, scale: float = 1.0) -> str:
    """Hex shield + linear AS bar (core brand mark)."""
    s = scale
    # Hex vertices relative
    pts = []
    import math

    for i in range(6):
        ang = math.radians(60 * i - 30)
        pts.append((cx + 48 * s * math.cos(ang), cy + 48 * s * math.sin(ang)))
    poly = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
    return f"""
  <polygon points="{poly}" fill="#0b1428" fill-opacity="0.85" stroke="#00e5ff" stroke-width="{3 * s}" filter="url(#neon-glow)"/>
  <polygon points="{poly}" fill="none" stroke="#38bdf8" stroke-width="{1 * s}" stroke-opacity="0.6"/>
  <rect x="{cx - 28 * s}" y="{cy - 6 * s}" width="{56 * s}" height="{12 * s}" rx="{2 * s}" fill="#00e5ff" fill-opacity="0.15" stroke="#00ff9d" stroke-width="{2 * s}"/>
  <circle cx="{cx}" cy="{cy}" r="{6 * s}" fill="#00e5ff" filter="url(#neon-glow)"/>
"""


def render_hero() -> None:
    body = f"""
{emblem(140, 180, 1.6)}
  <text x="280" y="130" fill="#f0f6fc" font-family="{FONT_MONO}" font-size="64" font-weight="700" letter-spacing="4">bilkrnl</text>
  <text x="284" y="172" fill="#94a3b8" font-family="{FONT_UI}" font-size="20">UEFI soft-ISA enclaves · blast chamber · cap ratchet</text>
  <g transform="translate(284,210)">
    <rect x="0" y="0" width="150" height="36" rx="4" fill="#0b1428" stroke="#00ff9d" stroke-width="1.5"/>
    <text x="16" y="24" fill="#00ff9d" font-family="{FONT_MONO}" font-size="14">1A interpreter</text>
    <rect x="166" y="0" width="130" height="36" rx="4" fill="#0b1428" stroke="#00e5ff" stroke-width="1.5"/>
    <text x="182" y="24" fill="#00e5ff" font-family="{FONT_MONO}" font-size="14">2C chamber</text>
    <rect x="312" y="0" width="140" height="36" rx="4" fill="#0b1428" stroke="#ffb800" stroke-width="1.5"/>
    <text x="328" y="24" fill="#ffb800" font-family="{FONT_MONO}" font-size="14">1C deferred</text>
  </g>
  <text x="284" y="300" fill="#5eead4" font-family="{FONT_MONO}" font-size="14">scripts/verify.sh · aarch64 virt · BIL1 bytecode</text>
"""
    write_svg(DOCS_ASSETS / "logo.svg", canvas(1200, 360, body))
    rasterize(DOCS_ASSETS / "logo.svg", DOCS_ASSETS / "logo.png")


def render_icon(path: Path, size: int = 512) -> None:
    body = emblem(size // 2, size // 2, size / 160)
    write_svg(path, canvas(size, size, body))
    rasterize(path, path.with_suffix(".png"), dpi=96 if size <= 64 else 192)


def render_architecture() -> None:
    boxes = [
        (40, 60, 200, 80, "QEMU/KVM Host", "aarch64 virt + AAVMF"),
        (300, 60, 200, 80, "UEFI Boot", "BOOTAA64.EFI"),
        (560, 60, 220, 80, "bilkrnl TCB", "kernel/src/main.rs"),
        (40, 200, 220, 100, "SoftISA Vm", "isa.rs · linear AS"),
        (300, 200, 220, 100, "EnclaveSlot", "load · run · dispose"),
        (560, 200, 220, 100, "BlastChamber", "chamber_ratify 2C"),
        (300, 360, 220, 80, "RatifiedArtifact", "SHA-256 digests"),
    ]
    arrows = [
        (240, 100, 300, 100),
        (500, 100, 560, 100),
        (670, 140, 670, 200),
        (560, 250, 520, 250),
        (300, 250, 260, 250),
        (410, 300, 410, 360),
    ]
    parts = [
        f'<text x="40" y="36" fill="#f0f6fc" font-family="{FONT_UI}" font-size="22" font-weight="700">bilkrnl architecture</text>',
    ]
    for x, y, w, h, title, sub in boxes:
        parts.append(panel(x, y, w, h))
        parts.append(
            f'<text x="{x + 16}" y="{y + 32}" fill="#f0f6fc" font-family="{FONT_UI}" font-size="15" font-weight="700">{title}</text>'
        )
        parts.append(
            f'<text x="{x + 16}" y="{y + 56}" fill="#5eead4" font-family="{FONT_MONO}" font-size="12">{sub}</text>'
        )
    for x1, y1, x2, y2 in arrows:
        parts.append(
            f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#00e5ff" stroke-width="2.5" stroke-opacity="0.75"/>'
        )
    parts.append(
        f'<text x="40" y="500" fill="#94a3b8" font-family="{FONT_MONO}" font-size="12">Host may speculate · enclave semantics must not · docs/roadmap/1c-softcpu.md</text>'
    )
    write_svg(EXAMPLES / "architecture.svg", canvas(840, 540, "\n".join(parts)))
    rasterize(EXAMPLES / "architecture.svg", EXAMPLES / "architecture.png")


def render_blast_chamber() -> None:
    stages = [
        (40, 120, "BIL1 image", "BytecodeImage", "#38bdf8"),
        (200, 120, "Load", "enclave_load", "#00e5ff"),
        (360, 120, "Run SoftISA", "Vm::step/run", "#00e5ff"),
        (520, 120, "Dispose", "zeroize mem", "#ffb800"),
        (680, 120, "Ratify", "chamber_ratify", "#00ff9d"),
    ]
    parts = [
        f'<text x="40" y="40" fill="#f0f6fc" font-family="{FONT_UI}" font-size="22" font-weight="700">Blast chamber pipeline (2C)</text>',
        f'<text x="40" y="68" fill="#94a3b8" font-family="{FONT_UI}" font-size="14">Dispose then ratify — no trust that execution was safe</text>',
    ]
    for i, (x, y, title, sub, color) in enumerate(stages):
        parts.append(
            f'<rect x="{x}" y="{y}" width="140" height="90" rx="6" fill="#0b1428" fill-opacity="0.92" stroke="{color}" stroke-width="1.5" filter="url(#neon-glow)"/>'
        )
        parts.append(hud_reticles(x, y, 140, 90, 10, color))
        parts.append(
            f'<text x="{x + 12}" y="{y + 36}" fill="#f0f6fc" font-family="{FONT_UI}" font-size="14" font-weight="700">{title}</text>'
        )
        parts.append(
            f'<text x="{x + 12}" y="{y + 60}" fill="#5eead4" font-family="{FONT_MONO}" font-size="11">{sub}</text>'
        )
        if i < len(stages) - 1:
            parts.append(
                f'<line x1="{x + 140}" y1="{y + 45}" x2="{x + 160}" y2="{y + 45}" stroke="#00e5ff" stroke-width="2"/>'
            )
    parts.append(panel(40, 260, 780, 120, "Serial markers (QEMU)"))
    parts.append(
        f'<text x="56" y="320" fill="#5eead4" font-family="{FONT_MONO}" font-size="13">bilkrnl: ready → blast: done → chamber: RATIFIED &lt;hex64&gt; → cli: RATIFIED &lt;same&gt; → bilkrnl: halt</text>'
    )
    parts.append(
        f'<text x="56" y="350" fill="#94a3b8" font-family="{FONT_MONO}" font-size="12">TC-01 e2e · TC-02 serial parity · TC-14 zeroization</text>'
    )
    write_svg(EXAMPLES / "blast-chamber.svg", canvas(860, 420, "\n".join(parts)))
    rasterize(EXAMPLES / "blast-chamber.svg", EXAMPLES / "blast-chamber.png")


def render_verify_tiers() -> None:
    tiers = [
        ("smoke/boot", "#00e5ff", "bilkrnl: ready"),
        ("unit", "#38bdf8", "TC-03..07,09,11,13"),
        ("integration", "#38bdf8", "TC-10,14 dispose"),
        ("security", "#ff2a85", "TC-06,08 escape"),
        ("system", "#00ff9d", "TC-01,02 QEMU"),
        ("perf", "#ffb800", "TC-15 cycles"),
        ("torture", "#ff2a85", "TC-12 kill"),
        ("docs", "#00e5ff", "render-graphics"),
    ]
    parts = [
        f'<text x="40" y="40" fill="#f0f6fc" font-family="{FONT_UI}" font-size="22" font-weight="700">scripts/verify.sh tiers</text>',
        f'<text x="40" y="68" fill="#94a3b8" font-family="{FONT_UI}" font-size="14">Layered gates — docs refresh runs after build stages</text>',
    ]
    for i, (name, color, note) in enumerate(tiers):
        x = 40 + (i % 4) * 200
        y = 100 + (i // 4) * 120
        parts.append(
            f'<rect x="{x}" y="{y}" width="180" height="90" rx="6" fill="#0b1428" fill-opacity="0.92" stroke="{color}" stroke-width="1.5"/>'
        )
        parts.append(hud_reticles(x, y, 180, 90, 10, color))
        parts.append(
            f'<text x="{x + 14}" y="{y + 36}" fill="#f0f6fc" font-family="{FONT_MONO}" font-size="14" font-weight="700">{name}</text>'
        )
        parts.append(
            f'<text x="{x + 14}" y="{y + 62}" fill="#94a3b8" font-family="{FONT_UI}" font-size="12">{note}</text>'
        )
    write_svg(EXAMPLES / "verify-tiers.svg", canvas(860, 360, "\n".join(parts)))
    rasterize(EXAMPLES / "verify-tiers.svg", EXAMPLES / "verify-tiers.png")


def render_cap_ratchet() -> None:
    levels = [("None", 0), ("Compute", 1), ("IoOutBounded", 2)]
    parts = [
        f'<text x="40" y="40" fill="#f0f6fc" font-family="{FONT_UI}" font-size="22" font-weight="700">Capability poset / ratchet</text>',
        f'<text x="40" y="68" fill="#94a3b8" font-family="{FONT_UI}" font-size="14">weaken only — CapPoset::weaken · TC-09</text>',
    ]
    for i, (name, _) in enumerate(levels):
        y = 120 + i * 70
        parts.append(
            f'<rect x="80" y="{y}" width="280" height="50" rx="6" fill="#0b1428" stroke="#00e5ff" stroke-width="1.5"/>'
        )
        parts.append(
            f'<text x="100" y="{y + 32}" fill="#f0f6fc" font-family="{FONT_MONO}" font-size="16">{name}</text>'
        )
        if i < 2:
            parts.append(
                f'<text x="400" y="{y + 55}" fill="#00ff9d" font-family="{FONT_MONO}" font-size="13">↓ weaken OK</text>'
            )
    parts.append(
        f'<text x="400" y="160" fill="#ff2a85" font-family="{FONT_MONO}" font-size="13">↑ strengthen → CapError::RatchetViolation</text>'
    )
    parts.append(panel(80, 340, 520, 70))
    parts.append(
        f'<text x="96" y="382" fill="#5eead4" font-family="{FONT_MONO}" font-size="13">EFI / host caps not grantable to enclaves in V1</text>'
    )
    write_svg(EXAMPLES / "cap-ratchet.svg", canvas(700, 450, "\n".join(parts)))
    rasterize(EXAMPLES / "cap-ratchet.svg", EXAMPLES / "cap-ratchet.png")


def render_category_icons() -> None:
    icons = {
        "icon-isa": ("#00e5ff", "ISA"),
        "icon-enclave": ("#38bdf8", "ENC"),
        "icon-chamber": ("#00ff9d", "BLST"),
        "icon-uefi": ("#ffb800", "UEFI"),
    }
    for name, (color, label) in icons.items():
        body = f"""
  <rect x="8" y="8" width="48" height="48" rx="6" fill="#0b1428" stroke="{color}" stroke-width="2"/>
  <text x="32" y="38" text-anchor="middle" fill="{color}" font-family="{FONT_MONO}" font-size="11" font-weight="700">{label}</text>
"""
        path = ICONS / f"{name}.svg"
        write_svg(path, canvas(64, 64, body))
        rasterize(path, path.with_suffix(".png"), dpi=96)


def main() -> int:
    for d in (DOCS_ASSETS, EXAMPLES, ICONS, ROOT_ASSETS):
        d.mkdir(parents=True, exist_ok=True)
    render_hero()
    render_icon(DOCS_ASSETS / "icon.svg", 512)
    render_icon(ROOT_ASSETS / "logo.svg", 512)
    render_architecture()
    render_blast_chamber()
    render_verify_tiers()
    render_cap_ratchet()
    render_category_icons()
    print(f"render-graphics: wrote assets under {DOCS_ASSETS.relative_to(ROOT)} and assets/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
