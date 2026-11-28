/*
 * SoftISA C port matching bilkrnl-core::isa::Vm (opcodes, budgets, caps, digests).
 * Host harness + QEMU bil-softcpu device share this body.
 */
#ifndef BIL_ISA_H
#define BIL_ISA_H

#include <stddef.h>
#include <stdint.h>

#define BIL_PAGE_SIZE 4096u
#define BIL_MAX_LINEAR_BYTES (256u * BIL_PAGE_SIZE)
#define BIL_MAX_CODE_LEN (64u * 1024u)
#define BIL_HEADER_LEN 32u

#define BIL_OP_HALT 0x00u
#define BIL_OP_NOP 0x01u
#define BIL_OP_LOAD_IMM 0x02u
#define BIL_OP_LOAD 0x03u
#define BIL_OP_STORE 0x04u
#define BIL_OP_ADD_IMM 0x05u
#define BIL_OP_SUB_IMM 0x06u
#define BIL_OP_JMP 0x07u
#define BIL_OP_JZ 0x08u
#define BIL_OP_OUT 0x09u
#define BIL_OP_WEAKEN 0x0Au

#define BIL_CAP_NONE 0u
#define BIL_CAP_COMPUTE 1u
#define BIL_CAP_IO_OUT_BOUNDED 2u

#define BIL_EXIT_HALT 0u
#define BIL_EXIT_BUDGET_EXCEEDED 1u
#define BIL_EXIT_FAULT 2u

#define BIL_FAULT_ILLEGAL_OPCODE 0u
#define BIL_FAULT_PC_OOB 1u
#define BIL_FAULT_MEM_OOB 2u
#define BIL_FAULT_HOST_ESCAPE 3u
#define BIL_FAULT_MISSING_CAP 4u
#define BIL_FAULT_OUT_OVERFLOW 5u

/* step / run control codes */
#define BIL_STEP_CONTINUE 0
#define BIL_STEP_EXIT 1
#define BIL_STEP_FAULT 2

typedef struct BilBudget {
    uint64_t max_cycles;
    uint32_t max_out_bytes;
    uint16_t max_mem_pages;
} BilBudget;

typedef struct BilAccount {
    uint64_t cycles;
    uint32_t out_bytes;
    uint16_t mem_high_water_pages;
} BilAccount;

typedef struct BilResult {
    uint32_t exit_kind;     /* BIL_EXIT_* */
    uint32_t fault_detail;  /* disc<<8 | payload */
    uint64_t cycles;
    uint16_t mem_high_water;
    uint8_t caps_at_exit;
    uint8_t code_digest[32];
    uint8_t out_digest[32];
    uint8_t *out;
    uint32_t out_len;
    uint32_t out_cap;
} BilResult;

typedef struct BilVm {
    uint8_t *mem;
    uint32_t mem_bytes;
    uint32_t pc;
    uint8_t acc;
    uint8_t caps;
    uint8_t *out;
    uint32_t out_len;
    uint32_t out_cap;
    BilAccount account;
    BilBudget budget;
    uint8_t code_digest[32];
    uint32_t code_len;
    int live;
} BilVm;

/* Pack FaultKind into FAULT_DETAIL layout. */
static inline uint32_t bil_pack_fault(uint8_t disc, uint8_t payload)
{
    return ((uint32_t)disc << 8) | (uint32_t)payload;
}

int bil_cap_ok(uint8_t c);
int bil_cap_weaken(uint8_t from, uint8_t to, uint8_t *out);

/* Encode 32-byte BIL1 header + code into dst (dst_cap must fit). Returns total len or 0. */
uint32_t bil_image_encode(uint8_t *dst, uint32_t dst_cap, uint32_t entry,
                          const uint8_t *code, uint32_t code_len, uint8_t caps,
                          BilBudget budget);

/* Decode header fields from bil1 bytes. Returns 0 on success. */
int bil_image_decode(const uint8_t *bytes, uint32_t len, uint32_t *entry,
                     const uint8_t **code, uint32_t *code_len, uint8_t *caps,
                     BilBudget *budget);

/* Init Vm from code+budget. Returns 0 on success. */
int bil_vm_init(BilVm *vm, const uint8_t *code, uint32_t code_len, uint32_t entry,
                BilBudget budget, uint8_t caps);

void bil_vm_free(BilVm *vm);

/*
 * One instruction. Sets *exit_kind / *fault_detail when returning EXIT/FAULT.
 * Returns BIL_STEP_CONTINUE / BIL_STEP_EXIT / BIL_STEP_FAULT.
 */
int bil_vm_step(BilVm *vm, uint32_t *exit_kind, uint32_t *fault_detail);

/* Run until halt/budget/fault; fills result (owns out buffer copy). */
void bil_vm_run(BilVm *vm, BilResult *result);

void bil_vm_zeroize(BilVm *vm);

void bil_result_free(BilResult *result);

/* artifact_digest framing from enclave.rs */
void bil_artifact_digest(const BilResult *result, uint8_t out[32]);

#endif /* BIL_ISA_H */
