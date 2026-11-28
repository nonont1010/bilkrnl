# Diagrams catalog

Rendered SVG/PNG live in [`../assets/examples/`](../assets/examples/). Sources here feed [`scripts/render-graphics.py`](../../scripts/render-graphics.py) ([diagrams-as-code](https://github.com/bsommers/diagrams-as-code), `cyber_slate`).

| Stem | Script | Shows |
|------|--------|--------|
| `architecture` | [`architecture.py`](architecture.py) | Host → UEFI → TCB → SoftISA / Enclave / Chamber |
| `blast-chamber` | [`blast_chamber.py`](blast_chamber.py) | Load → Run → Dispose → Ratify sequence |
| `verify-tiers` | [`verify_tiers.py`](verify_tiers.py) | `scripts/verify.sh` gates |
| `cap-ratchet` | [`cap_ratchet.py`](cap_ratchet.py) | Capability weaken-only poset |
| `softisa` | [`softisa.py`](softisa.py) | 1A `Vm` · host MMIO wrap · 1C `bil_isa.c` |

Also: ASCII [`ascii_architecture.txt`](ascii_architecture.txt), Mermaid [`*.mmd`](.), draw.io [`*.drawio`](.).

```bash
python3 scripts/render-graphics.py
./scripts/verify.sh --only docs
```
