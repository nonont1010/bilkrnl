"""bilkrnl system architecture (QEMU → UEFI → TCB → SoftISA / Enclave / Chamber)."""
from __future__ import annotations

from diagrams_as_code import Diagram, Shape
from diagrams_as_code.output import render_all


THEME = "cyber_slate"


def diagram() -> Diagram:
    with Diagram("bilkrnl architecture", theme=THEME) as diag:
        with diag.group("Host") as g_host:
            qemu = diag.add(
                Shape("qemu", "QEMU/KVM Host", icon="server", note="aarch64 virt")
            )
            aavmf = diag.add(
                Shape("aavmf", "AAVMF UEFI", icon="chip", firmware="AAVMF_CODE.fd")
            )
            aavmf.below(qemu, margin=28)

        with diag.group("Trusted TCB") as g_tcb:
            boot = diag.add(
                Shape("boot", "bilkrnl EFI", icon="container", path="kernel/")
            )
            core = diag.add(
                Shape("core", "bilkrnl-core", icon="chip", path="bilkrnl-core/")
            )
            core.below(boot, margin=28)

        with diag.group("Enclave path") as g_enc:
            isa = diag.add(
                Shape(
                    "isa",
                    "SoftISA",
                    icon="chip",
                    step="Vm::step | bil_isa.c",
                )
            )
            slot = diag.add(
                Shape("slot", "EnclaveSlot", icon="lock", lifecycle="load/run/dispose")
            )
            slot.below(isa, margin=28)
            chamber = diag.add(
                Shape("chamber", "BlastChamber", icon="shield", mode="2C ratify")
            )
            chamber.below(slot, margin=28)

        with diag.group("Artifact") as g_out:
            art = diag.add(
                Shape("artifact", "RatifiedArtifact", icon="key", digest="SHA-256")
            )

        g_host.at(40.0, 60.0)
        g_tcb.right_of(g_host, margin=70)
        g_enc.right_of(g_tcb, margin=70)
        g_out.right_of(g_enc, margin=70)

        qemu >> diag.edge("boot", style="solid") >> aavmf
        aavmf >> diag.edge("BOOTAA64.EFI", style="solid") >> boot
        boot >> diag.edge("contracts", style="solid") >> core
        core >> diag.edge("BIL1", style="solid") >> isa
        isa >> diag.edge("run", style="solid") >> slot
        slot >> diag.edge("dispose", style="solid") >> chamber
        chamber >> diag.edge("RATIFIED", style="solid") >> art

        diag.caption(
            "Host may speculate · enclave semantics must not · 1C SoftISA opt-in (SOFTCPU=1)",
            small_caps=True,
        )
    return diag


def render(out_dir, stem: str = "architecture") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
