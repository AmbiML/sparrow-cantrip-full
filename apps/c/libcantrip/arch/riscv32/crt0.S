/*
 * Copyright 2022 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

    .section .text._start
    .align 2
    .globl _start
    .type _start, @function
_start:
    .option push
    .option norelax

    la gp, __global_pointer$
    la x4, _tls

    /* Setup __sel4_ipc_buffer */
    lui t1, 0
    add t1, t1, tp
    sw a0, 0(t1)

    addi sp, sp, -16
    sw a0, 12(sp)
    sw a1, 8(sp)
    sw a2, 4(sp)
    sw a3, 0(sp)

    .option pop

    j main

    .bss

    .section .bss
    .align 12
    .globl _tls
    .type _tls, tls_object
_tls:
    .ds 4096
