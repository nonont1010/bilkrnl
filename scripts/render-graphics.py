#!/usr/bin/env python3
"""Render bilkrnl docs graphics via bsommers/diagrams-as-code.

Creates/uses `.venv-docs` with diagrams-as-code from GitHub, then runs the
figure scripts under `docs/diagrams/`. See `docs/TOOLING.md`.
"""
from __future__ import annotations

import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VENV = ROOT / ".venv-docs"
VENV_PY = VENV / "bin" / "python"
EXAMPLES = ROOT / "docs" / "assets" / "examples"
ASSETS = ROOT / "docs" / "assets"
ROOT_ASSETS = ROOT / "assets"
DAC_GIT = "git+https://github.com/bsommers/diagrams-as-code.git"


def ensure_venv() -> Path:
    if VENV_PY.is_file():
        # Fast path: already importable?
        probe = subprocess.run(
            [str(VENV_PY), "-c", "import diagrams_as_code"],
            cwd=ROOT,
            capture_output=True,
        )
        if probe.returncode == 0:
            return VENV_PY
    uv = shutil.which("uv")
    if not uv:
        raise SystemExit("uv is required to provision .venv-docs (https://docs.astral.sh/uv/)")
    if not VENV.exists():
        subprocess.check_call([uv, "venv", str(VENV)], cwd=ROOT)
    subprocess.check_call(
        [uv, "pip", "install", "--python", str(VENV_PY), DAC_GIT],
        cwd=ROOT,
    )
    return VENV_PY


def check_png(path: Path) -> None:
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise SystemExit(f"bad PNG magic: {path}")


def check_svg(path: Path) -> None:
    ET.parse(path)


def main() -> int:
    py = ensure_venv()
    # Run renderer body inside the docs venv so imports resolve.
    body = r"""
from pathlib import Path
from docs.diagrams import architecture, blast_chamber, verify_tiers, cap_ratchet
from docs.diagrams.brand_tron_corn import render_brand

ROOT = Path(%r)
EXAMPLES = ROOT / "docs" / "assets" / "examples"
ASSETS = ROOT / "docs" / "assets"
ROOT_ASSETS = ROOT / "assets"
ICONS = ASSETS / "icons"
EXAMPLES.mkdir(parents=True, exist_ok=True)
ASSETS.mkdir(parents=True, exist_ok=True)
ROOT_ASSETS.mkdir(parents=True, exist_ok=True)
ICONS.mkdir(parents=True, exist_ok=True)

architecture.render(EXAMPLES, "architecture")
blast_chamber.render(EXAMPLES, "blast-chamber")
verify_tiers.render(EXAMPLES, "verify-tiers")
cap_ratchet.render(EXAMPLES, "cap-ratchet")

# TRON neon techno-corn brand (custom SVG; not diagrams-as-code shapes)
render_brand(ASSETS, ROOT_ASSETS, ICONS)

print("render-graphics: diagrams-as-code figures + TRON corn brand written under docs/assets/")
""" % (str(ROOT),)
    subprocess.check_call([str(py), "-c", body], cwd=ROOT)

    # Well-formedness gates (host python; stdlib only)
    for p in EXAMPLES.glob("*.svg"):
        check_svg(p)
    for p in EXAMPLES.glob("*.png"):
        check_png(p)
    check_svg(ASSETS / "logo.svg")
    check_png(ASSETS / "logo.png")
    check_svg(ASSETS / "icon.svg")
    check_png(ASSETS / "icon.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
