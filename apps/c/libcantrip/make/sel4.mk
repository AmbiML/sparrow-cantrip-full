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

LIBSEL4_SRC ?= $(ROOTDIR)/cantrip/kernel/libsel4
OUT_CANTRIP    ?= $(OUT)/cantrip/$(ARCH_PREFIX)/$(BUILD_TYPE)

INCLUDES += -I$(LIBSEL4_SRC)/arch_include/$(BASE_ARCH_NAME)
INCLUDES += -I$(LIBSEL4_SRC)/include
INCLUDES += -I$(LIBSEL4_SRC)/mode_include/$(ARCH_BITS)
INCLUDES += -I$(LIBSEL4_SRC)/sel4_arch_include/$(FULL_ARCH_NAME)
INCLUDES += -I$(OUT_CANTRIP)/kernel/gen_config
INCLUDES += -I$(OUT_CANTRIP)/libsel4/autoconf
INCLUDES += -I$(OUT_CANTRIP)/libsel4/gen_config/
INCLUDES += -I$(OUT_CANTRIP)/libsel4/include
INCLUDES += -I$(OUT_CANTRIP)/libsel4/sel4_arch_include/$(FULL_ARCH_NAME)
