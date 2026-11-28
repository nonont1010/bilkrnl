"""SoftISA bodies: 1A oracle, host MMIO protocol wrap, 1C C port."""
from __future__ import annotations

from diagrams_as_code import Diagram, Shape
from diagrams_as_code.output import render_all


THEME = "cyber_slate"


def diagram() -> Diagram:
    with Diagram("SoftISA bodies & parity", theme=THEME) as diag:
        with diag.group("Contract") as g_contract:
            bil1 = diag.add(
                Shape("bil1", "BIL1 image", icon="file", header="32B + code")
            )
            digest = diag.add(
                Shape(
                    "digest",
                    "EnclaveResult digests",
                    icon="key",
                    fields="code/out/artifact",
                )
            )
            digest.below(bil1, margin=28)

        with diag.group("1A oracle") as g_1a:
            vm = diag.add(
                Shape("vm", "isa::Vm", icon="chip", path="bilkrnl-core/src/isa.rs")
            )

        with diag.group("Host protocol") as g_proto:
            mmio = diag.add(
                Shape(
                    "mmio",
                    "softcpu Rust",
                    icon="server",
                    note="wraps Vm (not a body)",
                )
            )

        with diag.group("1C SoftISA body") as g_1c:
            cisa = diag.add(
                Shape(
                    "cisa",
                    "bil_isa.c",
                    icon="chip",
                    path="softcpu/qemu/",
                )
            )
            qdev = diag.add(
                Shape(
                    "qdev",
                    "bil-softcpu",
                    icon="container",
                    run="SOFTCPU=1",
                )
            )
            qdev.below(cisa, margin=28)

        g_contract.at(40.0, 50.0)
        g_1a.right_of(g_contract, margin=60)
        g_proto.right_of(g_1a, margin=60)
        g_1c.right_of(g_proto, margin=60)

        bil1 >> diag.edge("decode", style="solid") >> vm
        bil1 >> diag.edge("MMIO FIFO", style="dashed") >> mmio
        bil1 >> diag.edge("COMMIT_LOAD", style="solid") >> cisa
        mmio >> diag.edge("calls Vm", style="dashed") >> vm
        cisa >> diag.edge("SysBus", style="solid") >> qdev
        vm >> diag.edge("PARITY.md", style="solid") >> digest
        cisa >> diag.edge("test-isa", style="solid") >> digest

        diag.caption(
            "C SoftISA is the sole 1C body · Rust softcpu is MMIO protocol only · digests match PARITY.md",
            small_caps=True,
        )
    return diag


def render(out_dir, stem: str = "softisa") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
