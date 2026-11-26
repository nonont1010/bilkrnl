"""Capability poset / forward-only ratchet (TC-09)."""
from __future__ import annotations

from diagrams_as_code import Diagram, Shape
from diagrams_as_code.output import render_all


THEME = "cyber_slate"


def diagram() -> Diagram:
    with Diagram("Capability poset / ratchet", theme=THEME) as diag:
        none = diag.add(Shape("none", "Cap::None", icon="lock", level="0"))
        compute = diag.add(Shape("compute", "Cap::Compute", icon="chip", level="1"))
        io = diag.add(Shape("io", "Cap::IoOutBounded", icon="gateway", level="2"))
        err = diag.add(
            Shape("err", "RatchetViolation", icon="malware_scan", when="strengthen")
        )

        none.at(80.0, 220.0)
        compute.above(none, margin=50)
        io.above(compute, margin=50)
        err.right_of(compute, margin=90)

        io >> diag.edge("weaken OK", style="solid") >> compute
        compute >> diag.edge("weaken OK", style="solid") >> none
        none >> diag.edge("strengthen FORBIDDEN", style="dashed") >> err
        compute >> diag.edge("strengthen FORBIDDEN", style="dashed") >> err

        diag.caption(
            "CapPoset::weaken only · EFI/host caps not grantable to enclaves in V1 · TC-09",
            small_caps=True,
        )
    return diag


def render(out_dir, stem: str = "cap-ratchet") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
