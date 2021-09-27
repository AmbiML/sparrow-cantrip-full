#include <camkes.h>
#include <stdint.h>

#include "vc_top/vc_top.h"

#define CSR_OFFSET (void *)csr

#define VCTOP_REG(name) \
  *((volatile uint32_t *)(CSR_OFFSET + VC_TOP_##name##_REG_OFFSET))

void vctop_set_ctrl(uint32_t ctrl) {
  VCTOP_REG(CTRL) = ctrl;
}
