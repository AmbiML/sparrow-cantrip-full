#
# Copyright 2020, Data61, CSIRO (ABN 41 687 119 230)
#
# SPDX-License-Identifier: BSD-2-Clause
#

cmake_minimum_required(VERSION 3.8.2)
include_guard(GLOBAL)

# Force TLS-model to be compatible with CAmkES usage (see
# tools/seL4/camkes-tool/helpers/environment_flags.cmake).
# NB: we cannot use CARGO_OPTIONS, cargo will strip it out
set(RUSTFLAGS "-Z tls-model=local-exec" CACHE INTERNAL "rustc env flags")

set(CARGO_OPTIONS
  -Z unstable-options
  -Z avoid-dev-deps
  CACHE INTERNAL "cargo cmd line arguments")

# add_library but for rust libraries. Invokes cargo in the SOURCE_DIR that is provided,
# all build output is placed in BUILD_DIR or CMAKE_CURRENT_BINARY_DIR if BUILD_DIR isn't provided.
# lib_name: Name of library that is created
# SOURCE_DIR: source directory of cargo project
# BUILD_DIR: directory for cargo build output
# LIB_FILENAME: filename of library created by cargo
# DEPENDS: And target or file dependencies that need to be run before cargo
function(RustAddLibrary lib_name)
    cmake_parse_arguments(PARSE_ARGV 1 RUST "" "SOURCE_DIR;BUILD_DIR;LIB_FILENAME" "DEPENDS")
    if(NOT "${RUST_UNPARSED_ARGUMENTS}" STREQUAL "")
        message(FATAL_ERROR "Unknown arguments to RustAddLibrary ${RUST_UNPARSED_ARGUMENTS}")
    endif()
    if("${RUST_SOURCE_DIR}" STREQUAL "")
        message(FATAL_ERROR "SOURCE_DIR must be set for RustAddLibrary")
    endif()
    if("${RUST_LIB_FILENAME}" STREQUAL "")
        message(FATAL_ERROR "LIB_FILENAME must be set for RustAddLibrary")
    endif()
    if("${RUST_BUILD_DIR}" STREQUAL "")
        set(RUST_BUILD_DIR ${CMAKE_CURRENT_BINARY_DIR})
    endif()

    if("${RELEASE}")
        set(CARGO_RELEASE "--release")
    endif()

    add_custom_target(
        ${lib_name}_custom
        BYPRODUCTS
        ${RUST_BUILD_DIR}/${RUST_LIB_FILENAME}
        ${USES_TERMINAL_DEBUG}
        DEPENDS ${RUST_DEPENDS}
        WORKING_DIRECTORY ${RUST_SOURCE_DIR}
        COMMAND
            ${CMAKE_COMMAND} -E env RUSTFLAGS=${RUSTFLAGS}
            cargo "+$ENV{CANTRIP_RUST_VERSION}" build
            --target ${RUST_TARGET}
            --features ${RUST_GLOBAL_FEATURES}
            ${CARGO_OPTIONS} ${CARGO_RELEASE}
            --target-dir ${RUST_BUILD_DIR}
            --out-dir ${RUST_BUILD_DIR}
    )

    add_library(${lib_name} STATIC IMPORTED GLOBAL)
    set_property(
        TARGET ${lib_name}
        PROPERTY IMPORTED_LOCATION "${RUST_BUILD_DIR}/${RUST_LIB_FILENAME}"
    )
    add_dependencies(${lib_name} ${lib_name}_custom)
endfunction()
