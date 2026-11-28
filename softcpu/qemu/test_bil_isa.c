/*
 * Host SoftISA harness — PARITY.md fixtures (no QEMU link).
 * Build: make -C softcpu/qemu test-isa
 */
#include "bil_isa.h"
#include "sha256.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int g_fail;

static void hex_to_bytes(const char *hex, uint8_t out[32])
{
    int i;
    for (i = 0; i < 32; i++) {
        unsigned int b;
        sscanf(hex + i * 2, "%2x", &b);
        out[i] = (uint8_t)b;
    }
}

static void expect_hex(const char *name, const char *field, const uint8_t got[32],
                       const char *want_hex)
{
    uint8_t want[32];
    char got_hex[65];
    int i;
    hex_to_bytes(want_hex, want);
    for (i = 0; i < 32; i++) {
        sprintf(got_hex + i * 2, "%02x", got[i]);
    }
    got_hex[64] = 0;
    if (memcmp(got, want, 32) != 0) {
        fprintf(stderr, "FAIL %s %s\n  got  %s\n  want %s\n", name, field, got_hex,
                want_hex);
        g_fail++;
    } else {
        printf("  ok %s %s\n", name, field);
    }
}

static void expect_u64(const char *name, const char *field, uint64_t got, uint64_t want)
{
    if (got != want) {
        fprintf(stderr, "FAIL %s %s: got %llu want %llu\n", name, field,
                (unsigned long long)got, (unsigned long long)want);
        g_fail++;
    } else {
        printf("  ok %s %s=%llu\n", name, field, (unsigned long long)got);
    }
}

static void expect_u32(const char *name, const char *field, uint32_t got, uint32_t want)
{
    if (got != want) {
        fprintf(stderr, "FAIL %s %s: got %u want %u\n", name, field, got, want);
        g_fail++;
    } else {
        printf("  ok %s %s=%u\n", name, field, got);
    }
}

static void run_fixture(const char *name, const uint8_t *code, uint32_t code_len,
                        uint8_t caps, BilBudget budget, uint32_t want_exit,
                        uint32_t want_fault_disc, uint64_t want_cycles,
                        const uint8_t *want_out, uint32_t want_out_len,
                        const char *code_digest_hex, const char *out_digest_hex,
                        const char *artifact_digest_hex)
{
    BilVm vm;
    BilResult result;
    uint8_t art[32];

    printf("fixture %s\n", name);
    if (bil_vm_init(&vm, code, code_len, 0, budget, caps) != 0) {
        fprintf(stderr, "FAIL %s: vm_init\n", name);
        g_fail++;
        return;
    }
    bil_vm_run(&vm, &result);
    expect_u32(name, "exit_kind", result.exit_kind, want_exit);
    if (want_exit == BIL_EXIT_FAULT) {
        expect_u32(name, "fault_disc", (result.fault_detail >> 8) & 0xff, want_fault_disc);
    }
    expect_u64(name, "cycles", result.cycles, want_cycles);
    expect_u32(name, "out_len", result.out_len, want_out_len);
    if (want_out_len > 0) {
        if (result.out == NULL || memcmp(result.out, want_out, want_out_len) != 0) {
            fprintf(stderr, "FAIL %s out bytes mismatch\n", name);
            g_fail++;
        } else {
            printf("  ok %s out bytes\n", name);
        }
    }
    expect_hex(name, "code_digest", result.code_digest, code_digest_hex);
    expect_hex(name, "out_digest", result.out_digest, out_digest_hex);
    bil_artifact_digest(&result, art);
    expect_hex(name, "artifact_digest", art, artifact_digest_hex);

    bil_vm_zeroize(&vm);
    bil_vm_free(&vm);
    bil_result_free(&result);
}

int main(void)
{
    /* empty SHA-256 */
    static const char *EMPTY_SHA =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    /* 1. halt */
    {
        static const uint8_t code[] = { 0x02, 0x4f, 0x09, 0x02, 0x4b, 0x09,
                                        0x02, 0x0a, 0x09, 0x00 };
        static const uint8_t out[] = { 0x4f, 0x4b, 0x0a };
        BilBudget b = { 64, 8, 1 };
        run_fixture("halt", code, sizeof(code), BIL_CAP_IO_OUT_BOUNDED, b, BIL_EXIT_HALT, 0,
                    7, out, 3,
                    "0fe5a198c3048319789344d79fba856c45e178446346f1417fca0eed7cacf015",
                    "a12b7cb43c9d9134b5bb1b35e9096b66775d9e92e7611d1cc92b02edd6782a87",
                    "25351c265eba64629fc702c47cd44372035b3c53b6bf0a5b98f2e86cb1fe798d");
    }

    /* 2. budget_exceed */
    {
        static const uint8_t code[] = { 0x01, 0x01, 0x00 };
        BilBudget b = { 2, 0, 1 };
        run_fixture("budget_exceed", code, sizeof(code), BIL_CAP_COMPUTE, b,
                    BIL_EXIT_BUDGET_EXCEEDED, 0, 3, NULL, 0,
                    "9f44ac6acb8a37b00b615dc89259d306035be82dee7db2b472e4c48326195440",
                    EMPTY_SHA,
                    "f71a27b9d61afe8e0cd55e6cef3e2b7473567aa103a7a99ca15e0c2a85fd3600");
    }

    /* 3. host_escape_e0 */
    {
        static const uint8_t code[] = { 0xe0 };
        BilBudget b = { 8, 0, 1 };
        run_fixture("host_escape_e0", code, sizeof(code), BIL_CAP_IO_OUT_BOUNDED, b,
                    BIL_EXIT_FAULT, BIL_FAULT_HOST_ESCAPE, 1, NULL, 0,
                    "7d8c5da7fd418379048e430b33dc8ffcda739e44326b8a5d647dc0ad81ed2157",
                    EMPTY_SHA,
                    "23a1aa25890f331caef6315f58ae15868b873eeb6251be6c5811d88773ef52a5");
    }

    /* 4. out_fill_ok */
    {
        static const uint8_t code[] = { 0x02, 0x58, 0x09, 0x02, 0x58, 0x09,
                                        0x02, 0x58, 0x09, 0x00 };
        static const uint8_t out[] = { 0x58, 0x58, 0x58 };
        BilBudget b = { 10000, 3, 1 };
        run_fixture("out_fill_ok", code, sizeof(code), BIL_CAP_IO_OUT_BOUNDED, b,
                    BIL_EXIT_HALT, 0, 7, out, 3,
                    "4664a3e53cb9aa682d22efe1845164d5b99d0275a93da5ea5b9101f849b3cec9",
                    "9b38b8f5877f2395b4361c1f68c059078ee9c0c8b0cbb22c97d2906e011e40a3",
                    "cb49179ea76e938d34735d1e889343a00c8bad911b555b550f3beb32062853e5");
    }

    /* 5. out_fill_overflow */
    {
        static const uint8_t code[] = { 0x02, 0x58, 0x09, 0x02, 0x58, 0x09,
                                        0x02, 0x58, 0x09, 0x00 };
        static const uint8_t out[] = { 0x58, 0x58 };
        BilBudget b = { 10000, 2, 1 };
        run_fixture("out_fill_overflow", code, sizeof(code), BIL_CAP_IO_OUT_BOUNDED, b,
                    BIL_EXIT_FAULT, BIL_FAULT_OUT_OVERFLOW, 6, out, 2,
                    "4664a3e53cb9aa682d22efe1845164d5b99d0275a93da5ea5b9101f849b3cec9",
                    "ecc76c19c9f3c5108773d6c3a18a6c25c9bf1131c4e250b71213274e3b2b5d08",
                    "41b0e9d71ac13d020d637c4d6d3c195240e4521bd43403a680bbcefeb72110a4");
    }

    /* 6. mem_oob */
    {
        static const uint8_t code[] = { 0x02, 0xab, 0x04, 0x00, 0x10, 0x00 };
        BilBudget b = { 16, 0, 1 };
        run_fixture("mem_oob", code, sizeof(code), BIL_CAP_COMPUTE, b, BIL_EXIT_FAULT,
                    BIL_FAULT_MEM_OOB, 2, NULL, 0,
                    "1ebf65bafc3e70876abe010f188c5cac66883dca4f1bdbdfa45ae3f71cba05ea",
                    EMPTY_SHA,
                    "9412dc121dfd93894348578d2203f00f9ba5a2545cd49e00f028fa741a9804a8");
    }

    /* Smoke: empty SHA-256 */
    {
        uint8_t d[32];
        uint8_t want[32];
        bil_sha256((const uint8_t *)"", 0, d);
        hex_to_bytes(EMPTY_SHA, want);
        if (memcmp(d, want, 32) != 0) {
            fprintf(stderr, "FAIL sha256 empty\n");
            g_fail++;
        } else {
            printf("  ok sha256 empty\n");
        }
    }

    if (g_fail) {
        fprintf(stderr, "\n%d assertion(s) failed\n", g_fail);
        return 1;
    }
    printf("\nall SoftISA parity fixtures passed\n");
    return 0;
}
