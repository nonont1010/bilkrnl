"""Wide hero banner for README / Pages (diagrams-as-code brand strip)."""
from __future__ import annotations

from diagrams_as_code import Diagram, Shape
from diagrams_as_code.output import render_all


THEME = "cyber_slate"


def diagram() -> Diagram:
    with Diagram("bilkrnl", theme=THEME) as diag:
        with diag.group("bilkrnl") as g:
            mark = diag.add(
                Shape("mark", "bilkrnl", icon="shield", tag="UEFI · SoftISA · Chamber")
            )
            a = diag.add(Shape("a", "1A interpreter", icon="chip"))
            b = diag.add(Shape("b", "2C chamber", icon="lock"))
            c = diag.add(Shape("c", "1C deferred", icon="chip"))
            a.right_of(mark, margin=40)
            b.right_of(a, margin=40)
            c.right_of(b, margin=40)
        g.at(40.0, 40.0)
        mark >> diag.edge("BIL1", style="solid") >> a
        a >> diag.edge("dispose→ratify", style="solid") >> b
        b >> diag.edge("soft-CPU later", style="dashed") >> c
        diag.caption(
            "scripts/verify.sh · aarch64 virt · bsommers/bil specs · diagrams-as-code figures",
            small_caps=True,
        )
    return diag


def render(out_dir, stem: str = "logo") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
