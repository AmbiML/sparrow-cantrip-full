#! /bin/bash
#
# Copyright 2020 Google LLC
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

# Wrapper script for running cargo clippy check for preupload.

set -ex

if [[ -z ${ROOTDIR} ]]; then
  echo "Source build/setup.sh first"
  exit 1
fi

# TODO(sleffler): Replace the following with `make -C ${ROOTDIR}/build cantrip-clippy`
RUST_TARGET=${RUST_TARGET:-riscv32imac-unknown-none-elf}
case "${PLATFORM}" in
  sparrow)
    CONFIG_PLATFORM=CONFIG_PLAT_SPARROW
    ;;
  nexus)
    CONFIG_PLATFORM=CONFIG_PLAT_NEXUS
    ;;
  *)
    echo "Unsupported platform ${PLATFORM}"
    exit -1
esac

CANTRIP_OUT_DIR="${OUT}/cantrip/${PLATFORM}"
mkdir -p "${CANTRIP_OUT_DIR}/clippy"

# HACK: sel4-config needs a path to the kernel build which could be
# in debug or release. Check the debug kernel first.
export SEL4_OUT_DIR=${SEL4_OUT_DIR:-"${CANTRIP_OUT_DIR}/debug/kernel"}
if [[ ! -d "${SEL4_OUT_DIR}/gen_config" ]]; then
  export SEL4_OUT_DIR="${CANTRIP_OUT_DIR}/release/kernel"
  if [[ ! -d "${SEL4_OUT_DIR}/gen_config" ]]; then
    echo "No kernel build found at ${CANTRIP_OUT_DIR}; build cantrip-bundle-debug first."
    exit 1
  fi
fi

declare -a CRATE_LIST=(
  apps/system/components/DebugConsole
  apps/system/components/MailboxDriver
  apps/system/components/MemoryManager
  apps/system/components/MlCoordinator
  apps/system/components/ProcessManager
  apps/system/components/SDKRuntime
  apps/system/components/SecurityCoordinator
  apps/system/components/TimerService
  apps/system/components/UARTDriver
)

# Run clippy that fails at warnings.
for crate in ${CRATE_LIST[@]}; do
  pushd ${crate} > /dev/null
    ${CARGO_HOME}/bin/cargo +${CANTRIP_RUST_VERSION} clippy \
      -Z unstable-options -Z avoid-dev-deps \
      --target ${RUST_TARGET} --features=${CONFIG_PLATFORM} \
      --target-dir ${CANTRIP_OUT_DIR}/clippy -- \
          -D warnings \
          -A clippy::uninlined_format_args
  popd > /dev/null
done
