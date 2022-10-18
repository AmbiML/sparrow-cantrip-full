/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// This file is a barebones, minimal-dependency test application that simply
// derefrences a null pointer to kill itself. It's primary use case is to test
// out CantripOS' fault handlers.

#include <cantrip.h>

int main() {
  debug_printf("Goodbye, cruel world!\n");

  while (1) {
    char *p = 0x0;
    *p = 'g';
  }
}
