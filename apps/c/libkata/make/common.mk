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
else
	DEBUG :=
	OPT   := -O0  # TODO(jtgans): Actually optimize in a release build
endif

ROOTDIR    ?= $(MYDIR)
BUILD_ROOT ?= $(ROOTDIR)/out/cantrip/$(ARCH_PREFIX)/$(BUILD_TYPE)/apps

CC := $(ARCH_PREFIX)-gcc
AS := $(ARCH_PREFIX)-as
AR := $(ARCH_PREFIX)-ar
LD := $(ARCH_PREFIX)-gcc

CFLAGS := $(DEBUG) $(OPT) $(INCLUDES)
CFLAGS += -march=$(ARCH) -mabi=$(ABI)
CFLAGS += -std=gnu11 -nostdlib
CFLAGS += -ftls-model=local-exec

ASFLAGS := -march=$(ARCH) -mabi=$(ABI)
LDFLAGS := $(DEBUG) -nostartfiles -static -nostdlib
