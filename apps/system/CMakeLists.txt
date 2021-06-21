#
# Copyright 2018, Data61
# Commonwealth Scientific and Industrial Research Organisation (CSIRO)
# ABN 41 687 119 230.
#
# This software may be distributed and modified according to the terms of
# the BSD 2-Clause license. Note that NO WARRANTY is provided.
# See "LICENSE_BSD2.txt" for details.
#
# @TAG(DATA61_BSD)
#

cmake_minimum_required(VERSION 3.7.2)

project(system)

CAmkESAddImportPath(interfaces)

include(${CMAKE_CURRENT_LIST_DIR}/rust.cmake)

RustAddLibrary(
  cantrip_debug_console
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/DebugConsole
  TARGET "riscv32imc-unknown-none-elf"
  LIB_FILENAME libcantrip_debug_console.a
)

DeclareCAmkESComponent(DebugConsole
  LIBS cantrip_debug_console
)

DeclareCAmkESComponent(UartDriver
  SOURCES components/UartDriver/src/driver.c
)

DeclareCAmkESComponent(SeL4Debug
  SOURCES components/SeL4Debug/src/wrappers.c
)

DeclareCAmkESRootserver(system.camkes)
