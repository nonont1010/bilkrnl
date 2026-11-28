/*
 * bil-softcpu — SysBus MMIO soft-CPU for bilkrnl (DEVICE.md).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * SoftISA body: bil_isa.c (parity with bilkrnl-core::isa::Vm / PARITY.md).
 * Host escape opcodes 0xE0..0xEF never call host/UEFI services.
 */
#include "qemu/osdep.h"
#include "hw/qdev-properties.h"
#include "hw/sysbus.h"
#include "qemu/log.h"
#include "qemu/module.h"
#include "qapi/error.h"
#include "hw/misc/bil_softcpu.h"

#include "bil_softcpu_regs.h"
#include "bil_isa.h"
#include "sha256.h"

struct BilSoftCpuState {
    SysBusDevice parent_obj;
    MemoryRegion mmio;
    uint16_t max_pages;

    uint32_t status;
    uint32_t load_err;
    uint32_t img_len;
    uint8_t *staging;
    uint32_t staging_len;
    uint32_t staging_cap;

    BilVm vm;
    int vm_live;

    uint32_t exit_kind;
    uint32_t fault_detail;
    uint64_t cycles;
    uint16_t mem_high_water;
    uint8_t caps_at_exit;
    uint8_t *out;
    uint32_t out_len;
    uint32_t out_cap;
    uint32_t out_read_idx;
    uint8_t code_digest[32];
    uint8_t out_digest[32];
    uint32_t zero_verified;
};

static void clear_result_window(BilSoftCpuState *s)
{
    s->exit_kind = 0;
    s->fault_detail = 0;
    s->cycles = 0;
    s->mem_high_water = 0;
    s->caps_at_exit = 0;
    if (s->out) {
        memset(s->out, 0, s->out_cap);
    }
    s->out_len = 0;
    s->out_read_idx = 0;
    memset(s->code_digest, 0, 32);
    memset(s->out_digest, 0, 32);
}

static void free_vm(BilSoftCpuState *s)
{
    if (s->vm_live) {
        bil_vm_free(&s->vm);
        s->vm_live = 0;
    }
}

static uint8_t read_out_byte(BilSoftCpuState *s)
{
    if (s->out_read_idx < s->out_len && s->out) {
        uint8_t b = s->out[s->out_read_idx];
        s->out_read_idx++;
        return b;
    }
    return 0;
}

static void latch_from_vm(BilSoftCpuState *s, uint32_t exit_kind, uint32_t fault_detail)
{
    BilVm *vm = &s->vm;
    s->exit_kind = exit_kind;
    s->fault_detail = fault_detail;
    s->cycles = vm->account.cycles;
    s->mem_high_water = vm->account.mem_high_water_pages;
    s->caps_at_exit = vm->caps;
    memcpy(s->code_digest, vm->code_digest, 32);
    bil_sha256(vm->out, vm->out_len, s->out_digest);

    if (vm->out_len > s->out_cap) {
        uint8_t *n = g_realloc(s->out, vm->out_len);
        s->out = n;
        s->out_cap = vm->out_len;
    }
    if (vm->out_len > 0 && s->out) {
        memcpy(s->out, vm->out, vm->out_len);
    }
    s->out_len = vm->out_len;
    s->out_read_idx = 0;
    s->status = (exit_kind == BIL_EXIT_FAULT) ? BIL_SOFTCPU_STATUS_FAULTED
                                              : BIL_SOFTCPU_STATUS_DONE;
}

static void cmd_begin_load(BilSoftCpuState *s)
{
    if (s->status == BIL_SOFTCPU_STATUS_IDLE ||
        s->status == BIL_SOFTCPU_STATUS_DISPOSED) {
        if (s->staging) {
            memset(s->staging, 0, s->staging_cap);
        }
        s->staging_len = 0;
        s->load_err = BIL_SOFTCPU_LOAD_ERR_OK;
        s->zero_verified = 0;
        clear_result_window(s);
        free_vm(s);
        s->status = BIL_SOFTCPU_STATUS_LOADING;
    } else {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_BUSY;
    }
}

static void cmd_commit_load(BilSoftCpuState *s)
{
    uint32_t entry;
    const uint8_t *code;
    uint32_t code_len;
    uint8_t caps;
    BilBudget budget;

    if (s->status != BIL_SOFTCPU_STATUS_LOADING) {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_BUSY;
        return;
    }
    if (s->staging_len != s->img_len) {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_LEN;
        return;
    }
    if (bil_image_decode(s->staging, s->staging_len, &entry, &code, &code_len, &caps,
                         &budget) != 0) {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_IMAGE;
        return;
    }
    if (budget.max_mem_pages == 0 || budget.max_mem_pages > s->max_pages) {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_PAGES;
        return;
    }
    free_vm(s);
    if (bil_vm_init(&s->vm, code, code_len, entry, budget, caps) != 0) {
        s->load_err = BIL_SOFTCPU_LOAD_ERR_VM_INIT;
        return;
    }
    s->vm_live = 1;
    memcpy(s->code_digest, s->vm.code_digest, 32);
    s->load_err = BIL_SOFTCPU_LOAD_ERR_OK;
    s->status = BIL_SOFTCPU_STATUS_LOADED;
}

static void cmd_step(BilSoftCpuState *s)
{
    uint32_t exit_kind = 0;
    uint32_t fault_detail = 0;
    int st;

    if (s->status != BIL_SOFTCPU_STATUS_LOADED &&
        s->status != BIL_SOFTCPU_STATUS_RUNNING) {
        return;
    }
    if (!s->vm_live) {
        return;
    }
    s->status = BIL_SOFTCPU_STATUS_RUNNING;
    st = bil_vm_step(&s->vm, &exit_kind, &fault_detail);
    if (st != BIL_STEP_CONTINUE) {
        latch_from_vm(s, exit_kind, fault_detail);
    }
}

static void cmd_run(BilSoftCpuState *s)
{
    BilResult result;

    if (s->status != BIL_SOFTCPU_STATUS_LOADED || !s->vm_live) {
        return;
    }
    s->status = BIL_SOFTCPU_STATUS_RUNNING;
    bil_vm_run(&s->vm, &result);
    latch_from_vm(s, result.exit_kind, result.fault_detail);
    bil_result_free(&result);
}

static void cmd_reset_zeroize(BilSoftCpuState *s)
{
    switch (s->status) {
    case BIL_SOFTCPU_STATUS_IDLE:
        s->zero_verified = 0;
        break;
    case BIL_SOFTCPU_STATUS_LOADING:
    case BIL_SOFTCPU_STATUS_RUNNING:
        break;
    case BIL_SOFTCPU_STATUS_DISPOSED:
        s->zero_verified = 1;
        break;
    case BIL_SOFTCPU_STATUS_LOADED:
    case BIL_SOFTCPU_STATUS_DONE:
    case BIL_SOFTCPU_STATUS_FAULTED:
        if (s->vm_live) {
            bil_vm_zeroize(&s->vm);
            free_vm(s);
        }
        if (s->staging && s->staging_cap) {
            memset(s->staging, 0, s->staging_cap);
        }
        s->staging_len = 0;
        clear_result_window(s);
        s->img_len = 0;
        s->load_err = BIL_SOFTCPU_LOAD_ERR_OK;
        s->zero_verified = 1;
        s->status = BIL_SOFTCPU_STATUS_DISPOSED;
        break;
    default:
        break;
    }
}

static void strobe_cmd(BilSoftCpuState *s, uint32_t cmd)
{
    switch (cmd) {
    case BIL_SOFTCPU_CMD_BEGIN_LOAD:
        cmd_begin_load(s);
        break;
    case BIL_SOFTCPU_CMD_COMMIT_LOAD:
        cmd_commit_load(s);
        break;
    case BIL_SOFTCPU_CMD_STEP:
        cmd_step(s);
        break;
    case BIL_SOFTCPU_CMD_RUN:
        cmd_run(s);
        break;
    case BIL_SOFTCPU_CMD_RESET_ZEROIZE:
        cmd_reset_zeroize(s);
        break;
    default:
        break;
    }
}

static uint32_t peek_u32(BilSoftCpuState *s, hwaddr addr)
{
    uint32_t val = 0;

    switch (addr) {
    case BIL_SOFTCPU_OFF_MAGIC:
        return BIL_SOFTCPU_MAGIC;
    case BIL_SOFTCPU_OFF_VERSION:
        return BIL_SOFTCPU_VERSION;
    case BIL_SOFTCPU_OFF_HW_CAPS:
        return (uint32_t)s->max_pages |
               ((uint32_t)BIL_SOFTCPU_MAX_CODE_BYTES << 16);
    case BIL_SOFTCPU_OFF_STATUS:
        return s->status;
    case BIL_SOFTCPU_OFF_LOAD_ERR:
        return s->load_err;
    case BIL_SOFTCPU_OFF_IMG_LEN:
        return s->img_len;
    case BIL_SOFTCPU_OFF_EXIT_KIND:
        return s->exit_kind;
    case BIL_SOFTCPU_OFF_FAULT_DETAIL:
        return s->fault_detail;
    case BIL_SOFTCPU_OFF_CYCLES_LO:
        return (uint32_t)(s->cycles & 0xffffffffu);
    case BIL_SOFTCPU_OFF_CYCLES_HI:
        return (uint32_t)(s->cycles >> 32);
    case BIL_SOFTCPU_OFF_OUT_LEN:
        return s->out_len;
    case BIL_SOFTCPU_OFF_OUT_READ_IDX:
        return s->out_read_idx;
    case BIL_SOFTCPU_OFF_ZERO_VERIFIED:
        return s->zero_verified;
    case BIL_SOFTCPU_OFF_MEM_HIGH_WATER:
        return s->mem_high_water;
    default:
        if (addr >= BIL_SOFTCPU_OFF_CODE_DIGEST &&
            addr + 4 <= BIL_SOFTCPU_OFF_CODE_DIGEST + 32 &&
            (addr & 3) == 0) {
            memcpy(&val, s->code_digest + (addr - BIL_SOFTCPU_OFF_CODE_DIGEST), 4);
            return val;
        }
        if (addr >= BIL_SOFTCPU_OFF_OUT_DIGEST &&
            addr + 4 <= BIL_SOFTCPU_OFF_OUT_DIGEST + 32 &&
            (addr & 3) == 0) {
            memcpy(&val, s->out_digest + (addr - BIL_SOFTCPU_OFF_OUT_DIGEST), 4);
            return val;
        }
        return 0;
    }
}

static uint64_t bil_softcpu_read(void *opaque, hwaddr addr, unsigned size)
{
    BilSoftCpuState *s = opaque;
    uint64_t val = 0;

    if (addr + size > BIL_SOFTCPU_MMIO_SIZE) {
        return 0;
    }

    if (size == 1) {
        if (addr == BIL_SOFTCPU_OFF_CAPS_AT_EXIT) {
            return s->caps_at_exit;
        }
        if (addr == BIL_SOFTCPU_OFF_OUT_DATA) {
            return read_out_byte(s);
        }
        if (addr >= BIL_SOFTCPU_OFF_CODE_DIGEST &&
            addr < BIL_SOFTCPU_OFF_CODE_DIGEST + 32) {
            return s->code_digest[addr - BIL_SOFTCPU_OFF_CODE_DIGEST];
        }
        if (addr >= BIL_SOFTCPU_OFF_OUT_DIGEST &&
            addr < BIL_SOFTCPU_OFF_OUT_DIGEST + 32) {
            return s->out_digest[addr - BIL_SOFTCPU_OFF_OUT_DIGEST];
        }
        if (addr >= BIL_SOFTCPU_OFF_MEM_HIGH_WATER &&
            addr < BIL_SOFTCPU_OFF_MEM_HIGH_WATER + 2) {
            uint16_t hw = s->mem_high_water;
            return (hw >> ((addr - BIL_SOFTCPU_OFF_MEM_HIGH_WATER) * 8)) & 0xff;
        }
        /* LE byte view of aligned 32-bit regs (no OUT_DATA side effects). */
        {
            hwaddr base = addr & ~3ull;
            uint32_t w = peek_u32(s, base);
            return (w >> ((addr & 3) * 8)) & 0xff;
        }
    }

    if (size == 2 && addr == BIL_SOFTCPU_OFF_MEM_HIGH_WATER) {
        return s->mem_high_water;
    }

    if (size == 4) {
        if (addr == BIL_SOFTCPU_OFF_OUT_DATA) {
            uint8_t buf[4];
            int i;
            for (i = 0; i < 4; i++) {
                buf[i] = read_out_byte(s);
            }
            memcpy(&val, buf, 4);
            return val;
        }
        return peek_u32(s, addr);
    }

    return 0;
}

static void push_staging_byte(BilSoftCpuState *s, uint8_t b)
{
    if (s->staging_len >= s->staging_cap) {
        uint32_t ncap = s->staging_cap ? s->staging_cap * 2 : 256;
        if (ncap > BIL_HEADER_LEN + BIL_MAX_CODE_LEN) {
            ncap = BIL_HEADER_LEN + BIL_MAX_CODE_LEN;
        }
        if (s->staging_len >= ncap) {
            return;
        }
        s->staging = g_realloc(s->staging, ncap);
        s->staging_cap = ncap;
    }
    s->staging[s->staging_len++] = b;
}

static void bil_softcpu_write(void *opaque, hwaddr addr, uint64_t value,
                              unsigned size)
{
    BilSoftCpuState *s = opaque;

    if (addr + size > BIL_SOFTCPU_MMIO_SIZE) {
        return;
    }

    if (addr == BIL_SOFTCPU_OFF_IMG_FIFO &&
        s->status == BIL_SOFTCPU_STATUS_LOADING) {
        push_staging_byte(s, (uint8_t)(value & 0xff));
        return;
    }

    if (size == 4) {
        switch (addr) {
        case BIL_SOFTCPU_OFF_CMD:
            strobe_cmd(s, (uint32_t)value);
            break;
        case BIL_SOFTCPU_OFF_IMG_LEN:
            if (s->status == BIL_SOFTCPU_STATUS_IDLE ||
                s->status == BIL_SOFTCPU_STATUS_LOADING ||
                s->status == BIL_SOFTCPU_STATUS_DISPOSED) {
                s->img_len = (uint32_t)value;
            }
            break;
        case BIL_SOFTCPU_OFF_OUT_READ_IDX:
            if (s->status == BIL_SOFTCPU_STATUS_DONE ||
                s->status == BIL_SOFTCPU_STATUS_FAULTED) {
                s->out_read_idx = (uint32_t)value;
            }
            break;
        default:
            break;
        }
    }
}

static const MemoryRegionOps bil_softcpu_ops = {
    .read = bil_softcpu_read,
    .write = bil_softcpu_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .valid = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
    .impl = {
        .min_access_size = 1,
        .max_access_size = 4,
    },
};

static void bil_softcpu_realize(DeviceState *dev, Error **errp)
{
    BilSoftCpuState *s = BIL_SOFTCPU(dev);

    if (s->max_pages == 0 || s->max_pages > BIL_SOFTCPU_DEFAULT_MAX_PAGES) {
        error_setg(errp, "bil-softcpu.max-pages must be in 1..%u",
                   BIL_SOFTCPU_DEFAULT_MAX_PAGES);
        return;
    }

    memory_region_init_io(&s->mmio, OBJECT(s), &bil_softcpu_ops, s,
                          TYPE_BIL_SOFTCPU, BIL_SOFTCPU_MMIO_SIZE);
    sysbus_init_mmio(SYS_BUS_DEVICE(s), &s->mmio);
}

static void bil_softcpu_reset_hold(Object *obj, ResetType type)
{
    BilSoftCpuState *s = BIL_SOFTCPU(obj);
    (void)type;

    free_vm(s);
    if (s->staging) {
        memset(s->staging, 0, s->staging_cap);
    }
    s->staging_len = 0;
    s->img_len = 0;
    s->load_err = 0;
    s->status = BIL_SOFTCPU_STATUS_IDLE;
    s->zero_verified = 0;
    clear_result_window(s);
}

static void bil_softcpu_finalize(Object *obj)
{
    BilSoftCpuState *s = BIL_SOFTCPU(obj);
    free_vm(s);
    g_free(s->staging);
    g_free(s->out);
}

static void bil_softcpu_init(Object *obj)
{
    BilSoftCpuState *s = BIL_SOFTCPU(obj);
    s->max_pages = BIL_SOFTCPU_DEFAULT_MAX_PAGES;
    s->status = BIL_SOFTCPU_STATUS_IDLE;
}

static const Property bil_softcpu_props[] = {
    DEFINE_PROP_UINT16("max-pages", BilSoftCpuState, max_pages,
                       BIL_SOFTCPU_DEFAULT_MAX_PAGES),
};

static void bil_softcpu_class_init(ObjectClass *oc, const void *data)
{
    DeviceClass *dc = DEVICE_CLASS(oc);
    ResettableClass *rc = RESETTABLE_CLASS(oc);
    (void)data;

    device_class_set_props(dc, bil_softcpu_props);
    dc->realize = bil_softcpu_realize;
    dc->desc = "bilkrnl SoftISA MMIO device (bil-softcpu)";
    dc->user_creatable = true;
    rc->phases.hold = bil_softcpu_reset_hold;
}

static const TypeInfo bil_softcpu_info = {
    .name = TYPE_BIL_SOFTCPU,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(BilSoftCpuState),
    .instance_init = bil_softcpu_init,
    .instance_finalize = bil_softcpu_finalize,
    .class_init = bil_softcpu_class_init,
};

static void bil_softcpu_register_types(void)
{
    type_register_static(&bil_softcpu_info);
}

type_init(bil_softcpu_register_types)
