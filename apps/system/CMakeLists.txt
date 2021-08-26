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

includeGlobalComponents()
include(${CMAKE_CURRENT_LIST_DIR}/rust.cmake)

RustAddLibrary(
  cantrip_debug_console
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/DebugConsole
  TARGET "riscv32imc-unknown-none-elf"
  LIB_FILENAME libcantrip_debug_console.a
)

DeclareCAmkESComponent(DebugConsole
  LIBS cantrip_debug_console
  INCLUDES interfaces
)

RustAddLibrary(
  cantrip_process_manager
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/ProcessManager
  TARGET "riscv32imc-unknown-none-elf"
  LIB_FILENAME libcantrip_process_manager.a
)

DeclareCAmkESComponent(ProcessManager
  LIBS cantrip_process_manager
  INCLUDES interfaces
)

RustAddLibrary(
  cantrip_ml_coordinator
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/MlCoordinator
  TARGET "riscv32imc-unknown-none-elf"
  LIB_FILENAME libcantrip_ml_coordinator.a
)

DeclareCAmkESComponent(MlCoordinator
  LIBS cantrip_ml_coordinator
  INCLUDES interfaces
)

DeclareCAmkESComponent(LogFibonacci
  SOURCES
  components/LogFibonacci/src/main.c
)

DeclareCAmkESComponent(OpenTitanUARTDriver
  SOURCES
  components/OpenTitanUARTDriver/src/driver.c
  components/OpenTitanUARTDriver/src/circular_buffer.c
  INCLUDES
  opentitan-gen/include
  components/OpenTitanUARTDriver/include
)

DeclareCAmkESComponent(SeL4Debug
  SOURCES components/SeL4Debug/src/wrappers.c
)

DeclareCAmkESComponent(VectorCoreDriver
  SOURCES components/VectorCoreDriver/src/driver.c
)

DeclareCAmkESRootserver(system.camkes)
