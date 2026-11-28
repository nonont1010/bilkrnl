/*
 * QEMU type header for -device bil-softcpu.
 * Not the design-only softcpu/include/bil_softcpu.h (that keeps #error).
 */
#ifndef HW_MISC_BIL_SOFTCPU_H
#define HW_MISC_BIL_SOFTCPU_H

#include "hw/sysbus.h"
#include "qom/object.h"

#define TYPE_BIL_SOFTCPU "bil-softcpu"

OBJECT_DECLARE_SIMPLE_TYPE(BilSoftCpuState, BIL_SOFTCPU)

#endif /* HW_MISC_BIL_SOFTCPU_H */
