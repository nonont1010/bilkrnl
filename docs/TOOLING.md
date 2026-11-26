# Documentation & visualization tooling

## Primary: diagrams-as-code

All architecture / workflow figures under `docs/assets/examples/` are produced by
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
