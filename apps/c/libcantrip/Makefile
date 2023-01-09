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

include make/common.mk

INCLUDES += -Iinclude

RUST_ARCH_DIR = ../../rust/libcantrip/arch

SRC_FILES += \
	printf.c \
	globals.c \
	$(RUST_ARCH_DIR)/riscv32/crt0.S \
	$(RUST_ARCH_DIR)/aarch64/crt0.S

INCLUDE_FILES := \
	include/cantrip.h

BUILD_DIR      := $(BUILD_ROOT)/libcantrip
BUILD_ARCH_DIR := $(BUILD_DIR)/arch/$(BUILD_ARCH)

OBJ_FILES := $(patsubst $(RUST_ARCH_DIR)/$(BUILD_ARCH)/%.S,$(BUILD_ARCH_DIR)/%.o,$(SRC_FILES))
OBJ_FILES := $(patsubst %.c,$(BUILD_DIR)/%.o,$(OBJ_FILES))

## Build Rules ###########################################

$(BUILD_DIR)/libcantrip.a: $(BUILD_DIR) $(OBJ_FILES) includes
	$(AR) rcs $@ $(OBJ_FILES)

includes: $(INCLUDE_FILES)
	mkdir -p $(BUILD_DIR)/include
	cp $(INCLUDE_FILES) $(BUILD_DIR)/include

$(BUILD_DIR)/%.o: %.c
	$(CC) $(CFLAGS) -Iinclude -c -o $@ $<

$(BUILD_DIR)/%.o: $(RUST_ARCH_DIR)/$(BUILD_ARCH)/%.S
	$(AS) $(ASFLAGS) -o $@ $<

$(BUILD_ARCH_DIR)/%.o: %.c
	$(CC) $(CFLAGS) -Iinclude -c -o $@ $<

$(BUILD_ARCH_DIR)/%.o: $(RUST_ARCH_DIR)/$(BUILD_ARCH)/%.S
	$(AS) $(ASFLAGS) -o $@ $<

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)
	mkdir -p $(BUILD_ARCH_DIR)

clean:
	rm -rf $(BUILD_DIR)

.PHONY: clean includes
