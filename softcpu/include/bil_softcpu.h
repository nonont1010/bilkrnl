/*
 * bil_softcpu.h — DESIGN ONLY / NOT COMPILED.
 *
 * Milestone 1C C API sketch for a future QEMU fork. This file is not a
 * translation unit of bilkrnl, bilkrnl-core, the ESP image, or
 * scripts/verify.sh. This repository has no Meson or CMake target that
 * builds it, and no .c provides these symbols.
 *
 * Guest code does not call these functions. bilkrnl.efi keeps
 * BytecodeImage / enclave_load / blast (see softcpu/qemu-device.md).
 * The #defines are the MMIO map in softcpu/DEVICE.md. The prototypes are
 * seams inside a future device model and a host parity harness.
 *
 * Non-negotiable, byte-identical with bilkrnl-core:
 *   - BIL1 encode/decode (32-byte header + code)
 *   - EnclaveResult fields and artifact_digest framing
 *   - dispose (this reset) then chamber_ratify; zeroize is not a digest
 */
#ifndef BIL_SOFTCPU_H
#define BIL_SOFTCPU_H

#if defined(BIL_SOFTCPU_BUILD) || defined(BILKRNL_BUILD)
#error "DESIGN ONLY: do not compile softcpu/include/bil_softcpu.h into bilkrnl or verify.sh"
#endif

#include <stddef.h>
#include <stdint.h>

/* --- MMIO window (DEVICE.md). Addresses are mmio_base + offset. --- */

#define BIL_SOFTCPU_MMIO_SIZE 0x10000u

#define BIL_SOFTCPU_OFF_MAGIC 0x0000u          /* u32 RO  0x314C4942 ("BIL1" LE) */
#define BIL_SOFTCPU_OFF_VERSION 0x0004u        /* u32 RO  0x00010000 */
#define BIL_SOFTCPU_OFF_HW_CAPS 0x0008u        /* u32 RO  pages[15:0], max code[31:16] */
#define BIL_SOFTCPU_OFF_STATUS 0x000cu         /* u32 RO */
#define BIL_SOFTCPU_OFF_CMD 0x0010u            /* u32 WO */
#define BIL_SOFTCPU_OFF_LOAD_ERR 0x0014u       /* u32 RO */
#define BIL_SOFTCPU_OFF_IMG_LEN 0x0018u        /* u32 RW */
#define BIL_SOFTCPU_OFF_IMG_FIFO 0x001cu       /* u32 WO; low 8 bits are the next image byte */
#define BIL_SOFTCPU_OFF_EXIT_KIND 0x0020u      /* u32 RO; artifact_bytes exit tag */
#define BIL_SOFTCPU_OFF_FAULT_DETAIL 0x0024u   /* u32 RO; kind in [15:8], payload in [7:0] */
#define BIL_SOFTCPU_OFF_CYCLES_LO 0x0028u      /* u32 RO */
#define BIL_SOFTCPU_OFF_CYCLES_HI 0x002cu      /* u32 RO */
#define BIL_SOFTCPU_OFF_MEM_HIGH_WATER 0x0030u /* u16 RO */
#define BIL_SOFTCPU_OFF_CAPS_AT_EXIT 0x0032u   /* u8  RO */
#define BIL_SOFTCPU_OFF_OUT_LEN 0x0034u        /* u32 RO */
#define BIL_SOFTCPU_OFF_OUT_READ_IDX 0x0038u   /* u32 RW */
#define BIL_SOFTCPU_OFF_OUT_DATA 0x003cu       /* u8/u32 RO; auto-increments the cursor */
#define BIL_SOFTCPU_OFF_CODE_DIGEST 0x0040u    /* 32 bytes RO; sha256(code) through 0x005f */
#define BIL_SOFTCPU_OFF_OUT_DIGEST 0x0060u     /* 32 bytes RO; sha256(out) through 0x007f */
#define BIL_SOFTCPU_OFF_ZERO_VERIFIED 0x0080u  /* u32 RO */

#define BIL_SOFTCPU_MAGIC 0x314c4942u
#define BIL_SOFTCPU_VERSION 0x00010000u

/* STATUS register (DEVICE.md STATUS_*). */
#define BIL_SOFTCPU_STATUS_IDLE 0u
#define BIL_SOFTCPU_STATUS_LOADING 1u
#define BIL_SOFTCPU_STATUS_LOADED 2u
#define BIL_SOFTCPU_STATUS_RUNNING 3u
#define BIL_SOFTCPU_STATUS_DONE 4u
#define BIL_SOFTCPU_STATUS_FAULTED 5u
#define BIL_SOFTCPU_STATUS_DISPOSED 6u

/* CMD strobe (DEVICE.md CMD_*). */
#define BIL_SOFTCPU_CMD_BEGIN_LOAD 1u
#define BIL_SOFTCPU_CMD_COMMIT_LOAD 2u
#define BIL_SOFTCPU_CMD_STEP 3u
#define BIL_SOFTCPU_CMD_RUN 4u
#define BIL_SOFTCPU_CMD_RESET_ZEROIZE 5u

/* EXIT_KIND / artifact_bytes tags. */
#define BIL_SOFTCPU_EXIT_HALT 0u
#define BIL_SOFTCPU_EXIT_BUDGET_EXCEEDED 1u
#define BIL_SOFTCPU_EXIT_FAULT 2u

/*
 * FAULT_DETAIL discriminants, FaultKind declaration order.
 * Payload byte is bits [7:0]; discriminant is bits [15:8].
 */
#define BIL_SOFTCPU_FAULT_ILLEGAL_OPCODE 0u
#define BIL_SOFTCPU_FAULT_PC_OOB 1u
#define BIL_SOFTCPU_FAULT_MEM_OOB 2u
#define BIL_SOFTCPU_FAULT_HOST_ESCAPE 3u
#define BIL_SOFTCPU_FAULT_MISSING_CAP 4u
#define BIL_SOFTCPU_FAULT_OUT_OVERFLOW 5u

#define BIL_SOFTCPU_DEFAULT_MAX_PAGES 256u

/* Opaque. Struct bodies stay in the fork. No layout is part of BIL1. */
typedef struct BilSoftCpu BilSoftCpu;
typedef struct BilSoftCpuImage BilSoftCpuImage;
typedef struct BilSoftCpuResult BilSoftCpuResult;

/*
 * Borrow a BytecodeImage::encode() vector (32-byte BIL1 header + code).
 * The bytes stay the caller's. This is not a second image format.
 * Returns NULL if bytes is NULL or len < 32. No body is linked in V1.
 */
BilSoftCpuImage *bil_softcpu_image_view(const uint8_t *bil1, uint32_t len);
void bil_softcpu_image_free(BilSoftCpuImage *image);

/*
 * BEGIN_LOAD + FIFO + COMMIT_LOAD. 0 means STATUS_LOADED.
 * A non-zero return leaves the device non-runnable and surfaces LoadError
 * (Image / VmInit / Busy). Once MAGIC selected this device, the caller
 * does not replay the image on the 1A interpreter.
 */
int bil_softcpu_load(BilSoftCpu *dev, const BilSoftCpuImage *image);

/*
 * CMD_RUN. Copies exit, cycles, mem_high_water, caps_at_exit, out, and
 * both digests into an opaque snapshot. The snapshot is the only result
 * that may outlive reset. Digests match the 1A oracle (softcpu/PARITY.md).
 * Returns 0 after STATUS_DONE or STATUS_FAULTED.
 */
int bil_softcpu_run(BilSoftCpu *dev, BilSoftCpuResult **out);
void bil_softcpu_result_free(BilSoftCpuResult *result);

/*
 * bil_softcpu_reset_zeroize — model seam for CMD_RESET_ZEROIZE (5).
 *
 * Implements DEVICE.md §5 / Vm::zeroize + EnclaveSlot::dispose. Call it
 * only after bil_softcpu_run has copied the EnclaveResult out. Blast order
 * stays load → run → return EnclaveResult → dispose → chamber_ratify.
 * chamber_ratify hashes that returned value in bilkrnl-core. This function
 * is not a digest input and has no ratify MMIO command.
 *
 * Legal while STATUS is LOADED, DONE, or FAULTED (software dispose accepts
 * Loaded or Ran; Ran covers halt, budget, and fault). The wipe is synchronous:
 *   - linear RAM (0 .. max_pages * 4096) set to 0x00
 *   - out buffer and IMG_FIFO staging set to 0x00; OUT_LEN = 0;
 *     OUT_READ_IDX = 0; a later OUT_DATA read must not return pre-dispose out
 *   - ACC = 0, PC = 0, CAPS = 0, cycle counters = 0, MEM_HIGH_WATER = 0
 *   - CODE_DIGEST, OUT_DIGEST, EXIT_KIND, FAULT_DETAIL, CAPS_AT_EXIT read 0
 * Then the model checks the backing bytes are 0x00, sets ZERO_VERIFIED = 1,
 * and stores STATUS_DISPOSED.
 *
 * From DISPOSED the call is idempotent and returns 0 (Dispose on an already
 * disposed slot is Ok). From IDLE with no image loaded it leaves STATUS_IDLE
 * and ZERO_VERIFIED = 0 and returns non-zero (DisposeError::NotFound
 * analogue). LOADING and RUNNING are rejected the same way.
 *
 * BEGIN_LOAD is allowed again from DISPOSED. The caller's BilSoftCpuResult
 * still holds the pre-wipe digests.
 *
 * Returns 0 when STATUS is DISPOSED and ZERO_VERIFIED is 1.
 * No definition is provided in this repository.
 */
int bil_softcpu_reset_zeroize(BilSoftCpu *dev);

#endif /* BIL_SOFTCPU_H */
