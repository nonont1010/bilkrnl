# Host SoftISA harness (no QEMU). PARITY.md fixtures.
CC ?= cc
CFLAGS ?= -O2 -Wall -Wextra -std=c11
OBJS = bil_isa.o sha256.o test_bil_isa.o

.PHONY: all test-isa clean

all: test-isa

test-isa: test_bil_isa
	./test_bil_isa

test_bil_isa: $(OBJS)
	$(CC) $(CFLAGS) -o $@ $(OBJS)

%.o: %.c bil_isa.h sha256.h
	$(CC) $(CFLAGS) -c -o $@ $<

clean:
	rm -f $(OBJS) test_bil_isa
