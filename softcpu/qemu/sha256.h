/* Public-domain SHA-256 (compact). */
#ifndef BIL_SHA256_H
#define BIL_SHA256_H

#include <stddef.h>
#include <stdint.h>

typedef struct BilSha256 {
    uint32_t state[8];
    uint64_t bitlen;
    uint8_t data[64];
    uint32_t datalen;
} BilSha256;

void bil_sha256_init(BilSha256 *ctx);
void bil_sha256_update(BilSha256 *ctx, const uint8_t *data, size_t len);
void bil_sha256_final(BilSha256 *ctx, uint8_t out[32]);
void bil_sha256(const uint8_t *data, size_t len, uint8_t out[32]);

#endif /* BIL_SHA256_H */
