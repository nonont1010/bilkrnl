"""scripts/verify.sh layered verification tiers."""
from __future__ import annotations

from diagrams_as_code import Diagram, Shape
from diagrams_as_code.output import render_all


THEME = "cyber_slate"


def diagram() -> Diagram:
    with Diagram("scripts/verify.sh tiers", theme=THEME, layout="auto") as diag:
        smoke = diag.add(Shape("smoke", "smoke/boot", icon="server", marker="bilkrnl: ready"))
        unit = diag.add(Shape("unit", "unit", icon="chip", tc="TC-03..07,09,11,13"))
        integ = diag.add(Shape("integ", "integration", icon="container", tc="TC-10,14"))
        sec = diag.add(Shape("sec", "security", icon="shield", tc="TC-06,08"))
        system = diag.add(Shape("system", "system", icon="gateway", tc="TC-01,02 QEMU"))
        perf = diag.add(Shape("perf", "perf", icon="dashboard", tc="TC-15 cycles"))
        torture = diag.add(Shape("torture", "torture", icon="malware_scan", tc="TC-12 kill"))
        bil = diag.add(Shape("bil", "bil-specs", icon="key", tool="bsommers/bil"))
        docs = diag.add(Shape("docs", "docs refresh", icon="document", tool="diagrams-as-code"))

        smoke >> diag.edge("", style="solid") >> unit
        unit >> diag.edge("", style="solid") >> integ
        integ >> diag.edge("", style="solid") >> sec
        sec >> diag.edge("", style="solid") >> system
        system >> diag.edge("", style="solid") >> perf
        perf >> diag.edge("", style="solid") >> torture
        torture >> diag.edge("", style="solid") >> bil
        bil >> diag.edge("", style="solid") >> docs

        diag.caption(
            "Optional bil-specs skip if bil CLI absent · docs always regenerated",
            small_caps=True,
        )
    return diag


def render(out_dir, stem: str = "verify-tiers") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
