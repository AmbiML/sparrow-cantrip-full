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

include $(MYDIR)/common.mk
include $(MYDIR)/libcantrip.mk

BUILD_DIR     := $(BUILD_ROOT)/$(APPNAME)
INTERMEDIATES := $(patsubst %.c,$(BUILD_DIR)/build/%.o,$(SOURCES))

$(BUILD_DIR)/$(APPNAME).elf: $(INTERMEDIATES) $(BUILD_ROOT)/libcantrip/libcantrip.a | $(BUILD_DIR)
	$(LD) $(LDFLAGS) -o $(BUILD_DIR)/$(APPNAME).elf $(INTERMEDIATES) $(LIBCANTRIP_LIBS) -lgcc

$(BUILD_DIR)/build/%.o: %.c $(BUILD_ROOT)/libcantrip/libcantrip.a | $(BUILD_DIR)
	$(CC) $(CFLAGS) $(LIBCANTRIP_INCLUDES) -c -o $@ $<

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)/build

clean:
	rm -rf $(BUILD_DIR)

.PHONY: clean

## libcantrip build linkage

$(BUILD_ROOT)/libcantrip/libcantrip.a:
	make -C $(MYDIR)/..
