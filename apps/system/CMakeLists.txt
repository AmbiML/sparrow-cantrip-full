# Copyright 2022 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

cmake_minimum_required(VERSION 3.7.2)

project(system)

CAmkESAddImportPath(
    components
    platforms/${KernelPlatform}/components
    interfaces
    platforms/${KernelPlatform}/interfaces
)

includeGlobalComponents()
include(${CMAKE_CURRENT_LIST_DIR}/rust.cmake)

# Note: the includes generated for each CAmkES component listed here are
# generated separately from the CMake build scripts. See also build/cantrip.mk for
# details, specifically the cantrip-component-headers target.

RustAddLibrary(
  cantrip_debug_console
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/DebugConsole
  LIB_FILENAME libcantrip_debug_console.a
)

DeclareCAmkESComponent(DebugConsole
  LIBS cantrip_debug_console
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  cantrip_memory_manager
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/MemoryManager
  LIB_FILENAME libcantrip_memory_manager.a
)

DeclareCAmkESComponent(MemoryManager
  LIBS cantrip_memory_manager
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  cantrip_process_manager
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/ProcessManager
  LIB_FILENAME libcantrip_process_manager.a
)

DeclareCAmkESComponent(ProcessManager
  LIBS cantrip_process_manager
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  cantrip_sdk_runtime
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/SDKRuntime
  LIB_FILENAME libcantrip_sdk_runtime.a
)

DeclareCAmkESComponent(SDKRuntime
  LIBS cantrip_sdk_runtime
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  cantrip_security_coordinator
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/components/SecurityCoordinator
  LIB_FILENAME libcantrip_security_coordinator.a
)

DeclareCAmkESComponent(SecurityCoordinator
  LIBS cantrip_security_coordinator
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

# Platform-specific components (typically drivers)
include(${CMAKE_CURRENT_LIST_DIR}/platforms/${KernelPlatform}/CMakeLists.txt OPTIONAL)

# Every platform has their own CAmkES assembly so services are built
# only when the needed drivers are present (e.g. TimerService).
DeclareCAmkESRootserver(platforms/${KernelPlatform}/system.camkes)
