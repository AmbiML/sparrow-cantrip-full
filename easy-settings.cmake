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

# NB: PLATFORM, CROSS_COMPILER_PREFIX, and RUST_TARGET are
#   expected to be set one the cmake command line
if(NOT DEFINED PLATFORM)
  message (SEND_ERROR "`PLATFORM` is not defined")
endif()
if(NOT DEFINED CROSS_COMPILER_PREFIX)
  message (SEND_ERROR "`CROSS_COMPILER_PREFIX` is not defined")
endif()
if(NOT DEFINED RUST_TARGET)
  message (SEND_ERROR "`RUST_TARGET` is not defined")
endif()

set(CAMKES_APP "system" CACHE STRING "The one and only CAmkES application in this project")
#set(CAPDL_LOADER_APP "capdl-loader-app" CACHE STRING "")
set(CAPDL_LOADER_APP "cantrip-os-rootserver" CACHE STRING "")

set(KernelIsMCS ON CACHE BOOL "Enable seL4 MCS support")
set(KernelPrinting ON CACHE BOOL "Enable seL4 console output support")
set(CAmkESDefaultHeapSize "8192" CACHE STRING "CAmkES per-component heap size (bytes)")

set(LibUtilsDefaultZfLogLevel 5 CACHE STRING "seL4 internal logging level (0-5).")
set(SIMULATION OFF CACHE BOOL "Whether to build simulate script")
set(RELEASE OFF CACHE BOOL "Performance optimized build")
# NB: UseRiscVBBL is set in the platform config

set(KernelNumDomains 1 CACHE STRING "How many scheduling domains to build for")
set(KernelDomainSchedule "${CMAKE_CURRENT_LIST_DIR}/kernel/round_robin_domain.c" CACHE INTERNAL "Domain scheduler algorithm")
