#include <camkes.h>
#include <stdint.h>

// TODO: Set offsets into memory based on `csr`.
#define CTRL (csr + 0x0)

void vctop_set_ctrl(uint32_t ctrl) {
    *((volatile uint32_t*)CTRL) = ctrl;
}
