/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// This file is a barebones, minimal-dependency test application.
// It prints the arguments passed in registers to the console
// using the seL4_DebugPutChar syscall and is intended as a starting
// point for low-level tests.

#include <cantrip.h>

int main(int a0, int a1, int a2, int a3) {
  debug_printf("\nI am a C app!\n");
  debug_printf("a0 %x a1 %x a2 %x a3 %x\n", a0, a1, a2, a3);
  debug_printf("__sel4_ipc_buffer %x\n", __sel4_ipc_buffer);

  debug_printf("Done, sleeping in WFI loop\n");
  while (1) {
    asm("wfi");
  }
}
