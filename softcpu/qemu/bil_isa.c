/*
 * SoftISA — C port of bilkrnl-core::isa::Vm / budget / cap / image / artifact_digest.
 */
#include "bil_isa.h"
#include "sha256.h"

#include <stdlib.h>
#include <string.h>

int bil_cap_ok(uint8_t c)
{
    return c <= BIL_CAP_IO_OUT_BOUNDED;
}

int bil_cap_weaken(uint8_t from, uint8_t to, uint8_t *out)
{
    if (!bil_cap_ok(to)) {
        return -1;
    }
    if (to > from) {
        return -1;
    }
    *out = to;
    return 0;
}

uint32_t bil_image_encode(uint8_t *dst, uint32_t dst_cap, uint32_t entry,
                          const uint8_t *code, uint32_t code_len, uint8_t caps,
                          BilBudget budget)
{
    uint32_t total = BIL_HEADER_LEN + code_len;
    if (dst == NULL || total > dst_cap || code_len > BIL_MAX_CODE_LEN) {
        return 0;
    }
    memset(dst, 0, BIL_HEADER_LEN);
    dst[0] = 'B';
    dst[1] = 'I';
    dst[2] = 'L';
    dst[3] = '1';
    memcpy(dst + 4, &entry, 4);
    memcpy(dst + 8, &code_len, 4);
    dst[12] = caps;
    memcpy(dst + 16, &budget.max_cycles, 8);
    memcpy(dst + 24, &budget.max_out_bytes, 4);
    memcpy(dst + 28, &budget.max_mem_pages, 2);
    if (code_len > 0 && code != NULL) {
        memcpy(dst + BIL_HEADER_LEN, code, code_len);
    }
    return total;
}

int bil_image_decode(const uint8_t *bytes, uint32_t len, uint32_t *entry,
                     const uint8_t **code, uint32_t *code_len, uint8_t *caps,
                     BilBudget *budget)
{
    uint32_t clen;
    uint8_t caps_raw;

    if (bytes == NULL || len < BIL_HEADER_LEN) {
        return -1;
    }
    if (bytes[0] != 'B' || bytes[1] != 'I' || bytes[2] != 'L' || bytes[3] != '1') {
        return -1;
    }
    memcpy(entry, bytes + 4, 4);
    memcpy(&clen, bytes + 8, 4);
    caps_raw = bytes[12];
    if (!bil_cap_ok(caps_raw)) {
        return -1;
    }
    *caps = caps_raw;
    memcpy(&budget->max_cycles, bytes + 16, 8);
    memcpy(&budget->max_out_bytes, bytes + 24, 4);
    memcpy(&budget->max_mem_pages, bytes + 28, 2);
    if (budget->max_mem_pages == 0) {
        return -1;
    }
    if (clen > BIL_MAX_CODE_LEN) {
        return -1;
    }
    if (len < BIL_HEADER_LEN + clen) {
        return -1;
    }
    if (clen > 0 && *entry >= clen) {
        return -1;
    }
    *code_len = clen;
    *code = bytes + BIL_HEADER_LEN;
    return 0;
}

static int charge_cycle(BilVm *vm)
{
    uint64_t next = vm->account.cycles + 1;
    if (next < vm->account.cycles) {
        next = UINT64_MAX; /* saturating */
    }
    if (next > vm->budget.max_cycles) {
        vm->account.cycles = next;
        return -1;
    }
    vm->account.cycles = next;
    return 0;
}

static int charge_out(BilVm *vm, uint32_t n)
{
    uint32_t next = vm->account.out_bytes + n;
    if (next < vm->account.out_bytes) {
        return -1;
    }
    if (next > vm->budget.max_out_bytes) {
        return -1;
    }
    vm->account.out_bytes = next;
    return 0;
}

static int note_addr(BilVm *vm, uint32_t addr)
{
    uint16_t page = (uint16_t)(addr / BIL_PAGE_SIZE);
    uint16_t pages_touched = (uint16_t)(page + 1u);
    if (pages_touched > vm->budget.max_mem_pages) {
        return -1;
    }
    if (pages_touched > vm->account.mem_high_water_pages) {
        vm->account.mem_high_water_pages = pages_touched;
    }
    return 0;
}

int bil_vm_init(BilVm *vm, const uint8_t *code, uint32_t code_len, uint32_t entry,
                BilBudget budget, uint8_t caps)
{
    uint32_t mem_bytes;

    memset(vm, 0, sizeof(*vm));
    mem_bytes = (uint32_t)budget.max_mem_pages * BIL_PAGE_SIZE;
    if (mem_bytes == 0 || mem_bytes > BIL_MAX_LINEAR_BYTES) {
        return -1;
    }
    if (code_len > 0 && entry >= code_len) {
        return -1;
    }
    if (code_len > mem_bytes) {
        return -1;
    }
    if (!bil_cap_ok(caps)) {
        return -1;
    }

    vm->mem = (uint8_t *)calloc(1, mem_bytes);
    if (vm->mem == NULL) {
        return -1;
    }
    vm->out_cap = budget.max_out_bytes > 0 ? budget.max_out_bytes : 1u;
    if (vm->out_cap > (1u << 20)) {
        vm->out_cap = 1u << 20;
    }
    /* Allow headroom for overflow tests that charge before push — buffer can hold max. */
    vm->out = (uint8_t *)calloc(1, vm->out_cap);
    if (vm->out == NULL) {
        free(vm->mem);
        vm->mem = NULL;
        return -1;
    }

    if (code_len > 0 && code != NULL) {
        memcpy(vm->mem, code, code_len);
    }
    bil_sha256(code ? code : (const uint8_t *)"", code_len, vm->code_digest);
    vm->mem_bytes = mem_bytes;
    vm->pc = entry;
    vm->acc = 0;
    vm->caps = caps;
    vm->out_len = 0;
    vm->budget = budget;
    vm->account.cycles = 0;
    vm->account.out_bytes = 0;
    vm->account.mem_high_water_pages = 0;
    vm->code_len = code_len;
    vm->live = 1;
    return 0;
}

void bil_vm_free(BilVm *vm)
{
    if (vm == NULL) {
        return;
    }
    free(vm->mem);
    free(vm->out);
    memset(vm, 0, sizeof(*vm));
}

static int read_u8(BilVm *vm, uint8_t *out)
{
    if (vm->pc >= vm->mem_bytes) {
        return -1;
    }
    *out = vm->mem[vm->pc];
    vm->pc = vm->pc + 1;
    return 0;
}

static int read_u16(BilVm *vm, uint16_t *out)
{
    uint8_t lo, hi;
    if (read_u8(vm, &lo) != 0 || read_u8(vm, &hi) != 0) {
        return -1;
    }
    *out = (uint16_t)lo | ((uint16_t)hi << 8);
    return 0;
}

static int mem_load(BilVm *vm, uint16_t addr, uint8_t *out)
{
    uint32_t a = addr;
    if (note_addr(vm, a) != 0) {
        return -1;
    }
    if (a >= vm->mem_bytes) {
        return -1;
    }
    *out = vm->mem[a];
    return 0;
}

static int mem_store(BilVm *vm, uint16_t addr, uint8_t val)
{
    uint32_t a = addr;
    if (note_addr(vm, a) != 0) {
        return -1;
    }
    if (a >= vm->mem_bytes) {
        return -1;
    }
    vm->mem[a] = val;
    return 0;
}

static int ensure_out_room(BilVm *vm)
{
    if (vm->out_len < vm->out_cap) {
        return 0;
    }
    {
        uint32_t ncap = vm->out_cap ? vm->out_cap * 2u : 16u;
        uint8_t *n = (uint8_t *)realloc(vm->out, ncap);
        if (n == NULL) {
            return -1;
        }
        vm->out = n;
        vm->out_cap = ncap;
    }
    return 0;
}

int bil_vm_step(BilVm *vm, uint32_t *exit_kind, uint32_t *fault_detail)
{
    uint8_t op;
    uint8_t imm;
    uint16_t addr;
    uint8_t to;

    *exit_kind = 0;
    *fault_detail = 0;

    if (charge_cycle(vm) != 0) {
        *exit_kind = BIL_EXIT_BUDGET_EXCEEDED;
        return BIL_STEP_EXIT;
    }

    if (vm->caps < BIL_CAP_COMPUTE) {
        *exit_kind = BIL_EXIT_FAULT;
        *fault_detail = bil_pack_fault(BIL_FAULT_MISSING_CAP, 0);
        return BIL_STEP_FAULT;
    }

    if (read_u8(vm, &op) != 0) {
        *exit_kind = BIL_EXIT_FAULT;
        *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
        return BIL_STEP_FAULT;
    }

    if (op >= 0xE0u && op <= 0xEFu) {
        *exit_kind = BIL_EXIT_FAULT;
        *fault_detail = bil_pack_fault(BIL_FAULT_HOST_ESCAPE, op);
        return BIL_STEP_FAULT;
    }

    switch (op) {
    case BIL_OP_HALT:
        *exit_kind = BIL_EXIT_HALT;
        return BIL_STEP_EXIT;
    case BIL_OP_NOP:
        return BIL_STEP_CONTINUE;
    case BIL_OP_LOAD_IMM:
        if (read_u8(vm, &imm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        vm->acc = imm;
        return BIL_STEP_CONTINUE;
    case BIL_OP_LOAD:
        if (read_u16(vm, &addr) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        if (mem_load(vm, addr, &imm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_MEM_OOB, 0);
            return BIL_STEP_FAULT;
        }
        vm->acc = imm;
        return BIL_STEP_CONTINUE;
    case BIL_OP_STORE:
        if (read_u16(vm, &addr) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        if (mem_store(vm, addr, vm->acc) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_MEM_OOB, 0);
            return BIL_STEP_FAULT;
        }
        return BIL_STEP_CONTINUE;
    case BIL_OP_ADD_IMM:
        if (read_u8(vm, &imm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        vm->acc = (uint8_t)(vm->acc + imm);
        return BIL_STEP_CONTINUE;
    case BIL_OP_SUB_IMM:
        if (read_u8(vm, &imm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        vm->acc = (uint8_t)(vm->acc - imm);
        return BIL_STEP_CONTINUE;
    case BIL_OP_JMP:
        if (read_u16(vm, &addr) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        if ((uint32_t)addr >= vm->mem_bytes) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        vm->pc = addr;
        return BIL_STEP_CONTINUE;
    case BIL_OP_JZ:
        if (read_u16(vm, &addr) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        if (vm->acc == 0) {
            if ((uint32_t)addr >= vm->mem_bytes) {
                *exit_kind = BIL_EXIT_FAULT;
                *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
                return BIL_STEP_FAULT;
            }
            vm->pc = addr;
        }
        return BIL_STEP_CONTINUE;
    case BIL_OP_OUT:
        if (vm->caps < BIL_CAP_IO_OUT_BOUNDED) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_MISSING_CAP, 0);
            return BIL_STEP_FAULT;
        }
        if (charge_out(vm, 1) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_OUT_OVERFLOW, 0);
            return BIL_STEP_FAULT;
        }
        if (ensure_out_room(vm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_OUT_OVERFLOW, 0);
            return BIL_STEP_FAULT;
        }
        vm->out[vm->out_len++] = vm->acc;
        return BIL_STEP_CONTINUE;
    case BIL_OP_WEAKEN:
        if (read_u8(vm, &imm) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_PC_OOB, 0);
            return BIL_STEP_FAULT;
        }
        if (!bil_cap_ok(imm)) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_ILLEGAL_OPCODE, imm);
            return BIL_STEP_FAULT;
        }
        to = imm;
        if (bil_cap_weaken(vm->caps, to, &to) != 0) {
            *exit_kind = BIL_EXIT_FAULT;
            *fault_detail = bil_pack_fault(BIL_FAULT_MISSING_CAP, 0);
            return BIL_STEP_FAULT;
        }
        vm->caps = to;
        return BIL_STEP_CONTINUE;
    default:
        *exit_kind = BIL_EXIT_FAULT;
        *fault_detail = bil_pack_fault(BIL_FAULT_ILLEGAL_OPCODE, op);
        return BIL_STEP_FAULT;
    }
}

void bil_result_free(BilResult *result)
{
    if (result == NULL) {
        return;
    }
    free(result->out);
    memset(result, 0, sizeof(*result));
}

static void fill_result(BilVm *vm, BilResult *result, uint32_t exit_kind,
                        uint32_t fault_detail)
{
    memset(result, 0, sizeof(*result));
    result->exit_kind = exit_kind;
    result->fault_detail = fault_detail;
    result->cycles = vm->account.cycles;
    result->mem_high_water = vm->account.mem_high_water_pages;
    result->caps_at_exit = vm->caps;
    memcpy(result->code_digest, vm->code_digest, 32);
    bil_sha256(vm->out, vm->out_len, result->out_digest);
    result->out_len = vm->out_len;
    result->out_cap = vm->out_len;
    if (vm->out_len > 0) {
        result->out = (uint8_t *)malloc(vm->out_len);
        if (result->out != NULL) {
            memcpy(result->out, vm->out, vm->out_len);
        } else {
            result->out_len = 0;
        }
    }
}

void bil_vm_run(BilVm *vm, BilResult *result)
{
    uint32_t exit_kind = 0;
    uint32_t fault_detail = 0;
    for (;;) {
        int st = bil_vm_step(vm, &exit_kind, &fault_detail);
        if (st == BIL_STEP_CONTINUE) {
            continue;
        }
        fill_result(vm, result, exit_kind, fault_detail);
        return;
    }
}

void bil_vm_zeroize(BilVm *vm)
{
    if (vm->mem != NULL && vm->mem_bytes > 0) {
        memset(vm->mem, 0, vm->mem_bytes);
    }
    if (vm->out != NULL && vm->out_cap > 0) {
        memset(vm->out, 0, vm->out_cap);
    }
    vm->out_len = 0;
    vm->acc = 0;
    vm->pc = 0;
    vm->caps = BIL_CAP_NONE;
    vm->account.cycles = 0;
    vm->account.out_bytes = 0;
    vm->account.mem_high_water_pages = 0;
}

void bil_artifact_digest(const BilResult *result, uint8_t out[32])
{
    /* exit_tag || cycles_le64 || out_len_le32 || out || code_digest || out_digest */
    uint8_t tag = (uint8_t)result->exit_kind;
    uint8_t cyl[8];
    uint8_t olen[4];
    BilSha256 ctx;
    uint32_t n = result->out_len;

    memcpy(cyl, &result->cycles, 8);
    memcpy(olen, &n, 4);

    bil_sha256_init(&ctx);
    bil_sha256_update(&ctx, &tag, 1);
    bil_sha256_update(&ctx, cyl, 8);
    bil_sha256_update(&ctx, olen, 4);
    if (result->out_len > 0 && result->out != NULL) {
        bil_sha256_update(&ctx, result->out, result->out_len);
    }
    bil_sha256_update(&ctx, result->code_digest, 32);
    bil_sha256_update(&ctx, result->out_digest, 32);
    bil_sha256_final(&ctx, out);
}
