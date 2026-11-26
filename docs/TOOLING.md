# Documentation & visualization tooling

## Brand: TRON neon techno-corn

Logo, favicon, and category icons are a custom **retro-futuristic TRON** mark:
neon corn cob nestled in a cyber husk/shell, plus a geometric path wordmark
`bilkrnl` (no webfonts — GitHub/offline safe).

- Generator: [`docs/diagrams/brand_tron_corn.py`](diagrams/brand_tron_corn.py)
- Emits: `docs/assets/logo.{svg,png}`, `docs/assets/icon.{svg,png}`,
  `assets/logo.{svg,png}`, `docs/assets/icons/icon-*.{svg,png}`
- Rasterized with ImageMagick (`magick` / `convert`)

## Primary: diagrams-as-code

Architecture / workflow figures under `docs/assets/examples/` are produced by
**[bsommers/diagrams-as-code](https://github.com/bsommers/diagrams-as-code)** (Apache-2.0).

- Source scripts: [`docs/diagrams/`](diagrams/)
- Orchestrator: [`scripts/render-graphics.py`](../scripts/render-graphics.py)
- Theme: `cyber_slate` (dark HUD; closest kit palette to the neon-grid docs site)
- Regenerated at the end of `scripts/verify.sh` (`refresh_docs`)

```bash
# one-shot
./scripts/verify.sh --only docs

# or
python3 scripts/render-graphics.py
```

Install the renderer if missing:

```bash
uv pip install --python "$(which python3)" "git+https://github.com/bsommers/diagrams-as-code.git"
# or: pip install "git+https://github.com/bsommers/diagrams-as-code.git"
```

## Available if needed: lumagrid

**[bsommers/lumagrid](https://github.com/bsommers/lumagrid)** is a layered React + TypeScript
widget library (bar/line/pie/scatter/area/column/heatmap/table/tree/choropleth/force, plus
D3/Recharts/Nivo adapters).

**Agents and maintainers may use lumagrid** whenever bilkrnl needs **interactive or web-based
charts / data widgets** (e.g. Pages dashboards, verify telemetry, cycle-budget trends). It is
**not** required for the current static SVG/PNG docs gallery — prefer diagrams-as-code for
architecture and sequence figures.

```bash
npm install lumagrid react react-dom
```

Do not pull lumagrid into the UEFI kernel or `bilkrnl-core` TCB.
