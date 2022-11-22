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

// NOLINT(build/header_guard)
#ifndef CANTRIP_H
#define CANTRIP_H

#include <kernel/gen_config.h>
#include <sel4/arch/syscalls.h>
#include <stdarg.h>

extern __thread seL4_IPCBuffer *__sel4_ipc_buffer;

#ifdef CONFIG_PRINTING
extern void _debug_printf(const char *fmt, ...);
#define debug_printf(args...) \
  do {                        \
    _debug_printf(args);      \
  } while (0)
#else
#define debug_printf(args...) \
  do {                        \
  } while (0)
#warning Apps will not log to console because CONFIG_PRINTING is not defined!
#endif  // CONFIG_PRINTING

#endif  // CANTRIP_H
