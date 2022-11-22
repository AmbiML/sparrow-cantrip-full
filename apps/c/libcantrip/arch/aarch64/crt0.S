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

/*
 * ProcessManager loads x0-x3 with _sel4_ipc_buffer and the information
 * needed to communicate with the SDKRuntime. Save those to the well-known
 * memory locations referenced by the SDK RPC.
 */
    .section .text._start
    .align 3
    .globl _start
    .type _start, @function
_start:
    ldr x30, =_tls          // NB: lr is an alias for x30
    msr tpidr_el0, x30      // setup static TLS

    str x0, [x30]           // setup __sel4_ipc_buffer at _tls+0

    ldr x30, =CANTRIP_SDK_ENDPOINT
    str x1, [x30]           // seL4_CPtr of SDKRuntime Endpoint

    ldr x30, =CANTRIP_SDK_FRAME
    str x2, [x30]           // seL4_CPtr of CANTRIP_SDK_PARAMS Frame object

    ldr x30, =CANTRIP_SDK_PARAMS
    str x3, [x30]           // virtual address of CANTRIP_SDK_PARAMS

    mov fp, #0
    mov lr, #0
    bl main
    /* NB: should not return */
1:  b 1b

    .section .bss
    .align 12
    .globl _tls
    .type _tls, tls_object
_tls:
    .ds.b 4096

    .align 3
    .global CANTRIP_SDK_ENDPOINT
CANTRIP_SDK_ENDPOINT:
    .ds.b 8

    .align 3
    .global CANTRIP_SDK_FRAME
CANTRIP_SDK_FRAME:
    .ds.b 8

    .align 3
    .global CANTRIP_SDK_PARAMS
CANTRIP_SDK_PARAMS:
    .ds.b 8
