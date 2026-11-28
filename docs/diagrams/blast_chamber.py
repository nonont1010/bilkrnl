"""Blast chamber pipeline as a UML sequence (dispose then ratify — 2C)."""
from __future__ import annotations

from diagrams_as_code import Diagram
from diagrams_as_code.output import render_all
from diagrams_as_code.sequence import SequenceDiagram


THEME = "cyber_slate"


def diagram() -> Diagram:
    seq = SequenceDiagram("Blast chamber pipeline (2C)", theme=THEME, autonumber=True)
    cli = seq.participant("Serial CLI / harness")
    slot = seq.participant("EnclaveSlot")
    vm = seq.participant("SoftISA Vm")
    chamber = seq.participant("BlastChamber")

    seq.message(cli, slot, "enclave_load(BIL1)", activate=True)
    seq.reply(slot, cli, "EnclaveId")
    seq.message(cli, slot, "enclave_run(id)", activate=True)
    seq.message(slot, vm, "Vm::run(budget)", activate=True)
    seq.reply(vm, slot, "EnclaveResult")
    seq.reply(slot, cli, "result")
    seq.message(cli, slot, "enclave_dispose(id)", activate=True)
    seq.note(slot, "zeroize linear RAM + out", side="right")
    seq.reply(slot, cli, "ok")
    seq.message(cli, chamber, "chamber_ratify(result, RatifySpec)", activate=True)
    with seq.alt("predicates hold") as outcome:
        seq.reply(chamber, cli, "RATIFIED + artifact_digest")
        outcome.branch("exit / budget / out / cap fail")
        seq.reply(chamber, cli, "Reject { reason }")
    diag = seq.build()
    diag.caption(
        "Markers: chamber: / chamber:rev4: / chamber:rot4: RATIFIED · cli: matches policy digest",
        small_caps=True,
    )
    return diag


def render(out_dir, stem: str = "blast-chamber") -> dict:
    return render_all(diagram(), out_dir, stem, formats=("svg", "png"))
