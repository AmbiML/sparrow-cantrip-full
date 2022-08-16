/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// This file is a barebones, minimal-dependency test application that simply
// derefrences a null pointer to kill itself. It's primary use case is to test
// out CantripOS' fault handlers.

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

int main(int a0, int a1, int a2, int a3) {
  seL4_DebugPutString("Goodbye, cruel world!\n");
  while (1) {
    char *p = 0x0;
    *p = 'g';
  }
}
