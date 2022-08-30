// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include <cantrip.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

#ifdef CONFIG_PRINTING
// only prints 32-bit "%x" hex values
void _debug_printf(const char *fmt, ...) {
  va_list args;
  va_start(args, fmt);
  for (; *fmt; fmt++) {
    if (*fmt == '%') {
      fmt++;
      if (*fmt == 'd') {
        uint32_t arg = va_arg(args, uint32_t);
        // TODO(sleffler): total hack
        int printing = 0;
        for (int d = 1000000000; d > 1; d /= 10) {
          int n = (arg / d) % 10;
          if (printing || n > 0) {
            seL4_DebugPutChar('0' + n);
            printing = 1;
          }
        }
        seL4_DebugPutChar('0' + (arg % 10));
      } else if (*fmt == 'x') {
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
}
#endif  // CONFIG_PRINTING
