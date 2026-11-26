"""Hero banner entrypoint — delegates to the TRON corn brand generator."""
from __future__ import annotations

from pathlib import Path

from docs.diagrams.brand_tron_corn import logo_banner_svg, write_pair


def render(out_dir, stem: str = "logo") -> dict:
    out = Path(out_dir)
    write_pair(logo_banner_svg(), out, stem)
    return {"svg": out / f"{stem}.svg", "png": out / f"{stem}.png"}
