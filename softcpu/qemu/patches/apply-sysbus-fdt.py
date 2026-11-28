#!/usr/bin/env python3
"""Idempotently patch QEMU hw/core/sysbus-fdt.c for bil-softcpu FDT binding."""
from __future__ import annotations

import pathlib
import sys

HELPER = r"""
static int add_bil_softcpu_node(SysBusDevice *sbdev, void *opaque)
{
    PlatformBusFDTData *data = opaque;
    PlatformBusDevice *pbus = data->pbus;
    const char *parent_node = data->pbus_node_name;
    void *fdt = data->fdt;
    uint64_t mmio_base;
    char *nodename;

    mmio_base = platform_bus_get_mmio_addr(pbus, sbdev, 0);
    nodename = g_strdup_printf("%s/bil-softcpu@%" PRIx64, parent_node, mmio_base);
    qemu_fdt_add_subnode(fdt, nodename);
    qemu_fdt_setprop_string(fdt, nodename, "compatible", "bil,softcpu");
    qemu_fdt_setprop_sized_cells(fdt, nodename, "reg",
                                 1, mmio_base,
                                 1, 0x10000);
    g_free(nodename);
    return 0;
}

"""


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    text = path.read_text()
    if "TYPE_BINDING(TYPE_BIL_SOFTCPU" in text and "add_bil_softcpu_node" in text:
        print("sysbus-fdt already patched")
        return 0
    if 'hw/misc/bil_softcpu.h' not in text:
        text = text.replace(
            '#include "hw/uefi/var-service-api.h"\n',
            '#include "hw/uefi/var-service-api.h"\n'
            '#include "hw/misc/bil_softcpu.h"\n',
            1,
        )
    marker = "static int no_fdt_node(SysBusDevice *sbdev, void *opaque)"
    if "add_bil_softcpu_node" not in text:
        if marker not in text:
            print("marker not found", file=sys.stderr)
            return 1
        text = text.replace(marker, HELPER + marker, 1)
    bind = "    TYPE_BINDING(TYPE_UEFI_VARS_SYSBUS, add_uefi_vars_node),"
    bind_new = (
        bind + "\n    TYPE_BINDING(TYPE_BIL_SOFTCPU, add_bil_softcpu_node),"
    )
    if "TYPE_BINDING(TYPE_BIL_SOFTCPU" not in text:
        if bind not in text:
            print("binding anchor not found", file=sys.stderr)
            return 1
        text = text.replace(bind, bind_new, 1)
    path.write_text(text)
    print(f"patched {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
