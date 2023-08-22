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

int main() {
  debug_printf("\nI am a C app!\n");

  debug_printf("Done, sleeping in infinite loop\n");
  while (1) {
    seL4_Yield();
  }
}
