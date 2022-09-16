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

MYDIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

BUILD_TYPE ?= debug
BUILD_ARCH ?= riscv32

include $(MYDIR)/arch/$(BUILD_ARCH).mk
include $(MYDIR)/sel4.mk

ifeq ($(BUILD_TYPE),debug)
	DEBUG := -g
	OPT   := -O0
    CARGO_OPTS :=
else
	DEBUG :=
	OPT   := -O0  # TODO(jtgans): Actually optimize in a release build
    CARGO_OPTS := --release
endif

ROOTDIR    ?= $(MYDIR)
BUILD_ROOT ?= $(ROOTDIR)/out/cantrip/$(ARCH_PREFIX)/$(BUILD_TYPE)/apps/rust

CC := $(ARCH_PREFIX)-gcc
AS := $(ARCH_PREFIX)-as
AR := $(ARCH_PREFIX)-ar
LD := $(ARCH_PREFIX)-gcc

CANTRIP_RUST_VERSION ?= nightly-2021-11-05
CARGO := cargo +${CANTRIP_RUST_VERSION}

CFLAGS := $(DEBUG) $(OPT) $(INCLUDES)
CFLAGS += -march=$(ARCH) -mabi=$(ABI)
CFLAGS += -std=gnu11 -nostdlib
CFLAGS += -ftls-model=${TLS_MODEL}

ASFLAGS := -march=$(ARCH) -mabi=$(ABI)
LDFLAGS := $(DEBUG) -nostartfiles -static -nostdlib

CARGO_OPTS += -Z unstable-options
CARGO_OPTS += -Z avoid-dev-deps
# XXX RUSTFLAGS is the only way to pass tls-model but seems to work w/o
#CARGO_OPTS += -Z tls-model=${TLS_MODEL}
CARGO_OPTS += --target ${FULL_ARCH_NAME}-unknown-none-elf
