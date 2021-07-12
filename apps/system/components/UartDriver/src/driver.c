// OpenTitan UART driver
//
// A programming guide for the hardware can be found at
// https://docs.opentitan.org/hw/ip/uart/doc/

#include <stdint.h>

#include <camkes.h>
#include <sel4/syscalls.h>

#include "opentitan/uart.h"

// Referenced by macros in the generated file opentitan/uart.h.
#define UART0_BASE_ADDR (void *)mem

// Frequency of the primary clock clk_i.
#define CLK_FIXED_FREQ_HZ (24ull * 1000 * 1000)

#define REG32(addr) *((volatile uint32_t *)(addr))

void uart__init() {
  // Computes NCO value corresponding to baud rate.
  // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
  uint64_t baud = 115200ull;
  uint64_t uart_ctrl_nco = ((uint64_t)baud << 20) / CLK_FIXED_FREQ_HZ;

  // Sets baud rate and enables TX and RX.
  REG32(UART_CTRL(0)) =
      ((uart_ctrl_nco & UART_CTRL_NCO_MASK) << UART_CTRL_NCO_OFFSET) |
      (1 << UART_CTRL_TX) | (1 << UART_CTRL_RX);

  // Resets TX and RX FIFOs.
  uint32_t fifo_ctrl = REG32(UART_FIFO_CTRL(0));
  REG32(UART_FIFO_CTRL(0)) =
      fifo_ctrl | UART_FIFO_CTRL_RXRST | UART_FIFO_CTRL_TXRST;

  // Disables interrupts.
  // TODO (mattharvey): Configure seL4 to dispatch UART interrupts to this
  // driver, enable here at least TX watermark and RX watermark, and add
  // handlers. For now, this driver spins to wait.
  REG32(UART_INTR_ENABLE(0)) = 0ul;
}

static int uart_rx_empty() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_RXEMPTY)) != 0;
  /*
   * A more direct adapation of the example from the Programmers Guide in
   *
   * https://docs.opentitan.org/hw/ip/uart/doc/
   *
   * would look like the below. (The example does not compile verbatim.) Using
   * the UART_STATUS register is simpler than this expression and seems
   * equivalent, according to the wording of the doc.
   *
   * Still, we'll keep this commented implementation until we gain some
   * confidence the simulation is happy with the simpler approach.
   *
  return (REG32(UART_FIFO_STATUS(0)) &
          (UART_FIFO_STATUS_RXLVL_MASK << UART_FIFO_STATUS_RXLVL_OFFSET)) >>
             UART_FIFO_STATUS_RXLVL_OFFSET ==
         0;
   */
}

static int uart_tx_ready() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_TXFULL)) == 0;
  /*
   * See similar comment in uart_rx_empty.
   *
  int32_t tx_fifo_capacity = 32;  // uart.h provides no define for this.
  return ((REG32(UART_FIFO_STATUS(0)) & UART_FIFO_STATUS_TXLVL_MASK) ==
          tx_fifo_capacity)
             ? 0
             : 1;
   */
}

void uart_rx(size_t n) {
  char *c = (char *)rx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  for (size_t i = 0; i < n && i < PAGE_SIZE; ++i) {
    while (uart_rx_empty()) {
      seL4_Yield();  // TODO(mattharvey): remove when interrupt-driven
    }
    *c = REG32(UART_RDATA(0)) & UART_RDATA_RDATA_MASK;
    ++c;
  }
}

void uart_tx(size_t n) {
  char *c = (char *)tx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  for (size_t i = 0; i < n && i < PAGE_SIZE; ++i) {
    while (!uart_tx_ready()) {
      seL4_Yield();  // TODO(mattharvey): remove when interrupt-driven
    }
    REG32(UART_WDATA(0)) = *c;
    ++c;
  }
}
