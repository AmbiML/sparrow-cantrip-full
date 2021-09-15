#include <camkes.h>
#include <stdint.h>

#include "vc_top/vc_top.h"

#define CTRL (csr + VC_TOP_CTRL_REG_OFFSET)

void vctop_set_ctrl(uint32_t ctrl) {
    *((volatile uint32_t*)CTRL) = ctrl;
}
