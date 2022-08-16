/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// This file is a barebones, minimal-dependency test application.
// It prints the arguments passed in registers to the console
// using the seL4_DebugPutChar syscall and is intended as a starting
// point for low-level tests.

#include <kernel/gen_config.h>
#include <sel4/arch/syscalls.h>
#include <stdarg.h>
#include <stdint.h>

__thread seL4_IPCBuffer *__sel4_ipc_buffer;

char minisel_tls[4096] __attribute__((__aligned__(4096)));

__attribute__((naked)) void _start() {
  asm volatile(
      ".option push                  \n"
      ".option norelax               \n"
      "la gp, __global_pointer$      \n"
      "la x4, minisel_tls            \n"
      "addi sp,sp,-16                \n"
      "sw a0, 12(sp)                 \n"
      "sw a1, 8(sp)                  \n"
      "sw a2, 4(sp)                  \n"
      "sw a3, 0(sp)                  \n"
      ".option pop                   \n"
      "j main                        \n");
}

// only prints 32-bit "%x" hex values
void minisel_printf(const char *fmt, ...) {
#if CONFIG_PRINTING
  va_list args;
  va_start(args, fmt);
  for (; *fmt; fmt++) {
    if (*fmt == '%') {
      fmt++;
      if (*fmt == 'x') {
        uint32_t arg = va_arg(args, uint32_t);
        for (int i = 7; i >= 0; i--) {
          int n = (arg >> (4 * i)) & 0xF;
          seL4_DebugPutChar(n > 9 ? 'A' + n - 10 : '0' + n);
        }
      }
    } else {
      seL4_DebugPutChar(*fmt);
    }
  }
  va_end(args);
#endif
}

int main(int a0, int a1, int a2, int a3) {
  minisel_printf("\na0 %x a1 %x a2 %x a3 %x\n", a0, a1, a2, a3);

  minisel_printf("Done, sleeping in WFI loop\n");
  while (1) {
    asm("wfi");
  }
}
