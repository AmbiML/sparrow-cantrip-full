#include <camkes.h>
#include <stdint.h>

#include "vc_top/vc_top.h"

#define CSR_OFFSET (void *)csr

#define VCTOP_REG(name) \
  *((volatile uint32_t *)(CSR_OFFSET + VC_TOP_##name##_REG_OFFSET))

// CAmkES initialization hook.
//
// Enables Interrupts.
//
void pre_init() {
  // Enables interrupts.
  VCTOP_REG(INTR_ENABLE) = (BIT(VC_TOP_INTR_COMMON_HOST_REQ_BIT) |
                            BIT(VC_TOP_INTR_ENABLE_FINISH_BIT) |
                            BIT(VC_TOP_INTR_COMMON_INSTRUCTION_FAULT_BIT) |
                            BIT(VC_TOP_INTR_COMMON_DATA_FAULT_BIT));
}

void vctop_set_ctrl(uint32_t ctrl) {
  VCTOP_REG(CTRL) = ctrl;
}

void host_req_handle(void) {
  // Also need to clear the INTR_STATE (write-1-to-clear).
  VCTOP_REG(INTR_STATE) = BIT(VC_TOP_INTR_STATE_HOST_REQ_BIT);
  seL4_Assert(host_req_acknowledge() == 0);
}

void finish_handle(void) {
  // Read main() return code and machine exception PC.
  vctop_return_update_result();
  // Also need to clear the INTR_STATE (write-1-to-clear).
  VCTOP_REG(INTR_STATE) = BIT(VC_TOP_INTR_STATE_FINISH_BIT);
  seL4_Assert(finish_acknowledge() == 0);
}

void instruction_fault_handle(void) {
  // Also need to clear the INTR_STATE (write-1-to-clear).
  VCTOP_REG(INTR_STATE) = BIT(VC_TOP_INTR_STATE_INSTRUCTION_FAULT_BIT);
  seL4_Assert(instruction_fault_acknowledge() == 0);
}

void data_fault_handle(void) {
  // Also need to clear the INTR_STATE (write-1-to-clear).
  VCTOP_REG(INTR_STATE) = BIT(VC_TOP_INTR_STATE_DATA_FAULT_BIT);
  seL4_Assert(data_fault_acknowledge() == 0);
}
