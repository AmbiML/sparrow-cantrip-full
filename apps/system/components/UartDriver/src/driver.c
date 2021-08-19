// OpenTitan UART driver
//
// A programming guide for the hardware can be found at
// https://docs.opentitan.org/hw/ip/uart/doc/

#include <assert.h>
#include <camkes.h>
#include <sel4/syscalls.h>
#include <stdint.h>

#include "opentitan/uart.h"

// Referenced by macros in the generated file opentitan/uart.h.
#define UART0_BASE_ADDR (void *)mmio_region

// Frequency of the primary clock clk_i.
#define CLK_FIXED_FREQ_HZ (48ull * 1000 * 1000)

#define REG32(addr) *((volatile uint32_t *)(addr))

#define UART_BUF_SIZE 512

static char rx_buf[UART_BUF_SIZE];
static char *rx_buf_end = rx_buf;  // guarded by rx_mutex

void pre_init() {
  // Computes NCO value corresponding to baud rate.
  // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
  seL4_CompileTimeAssert(UART_CTRL_NCO_MASK == 0xffff);
  uint64_t baud = 115200ull;
  uint64_t uart_ctrl_nco = ((uint64_t)baud << 20) / CLK_FIXED_FREQ_HZ;
  seL4_Assert(uart_ctrl_nco < 0xffff);

  // Sets baud rate and enables TX and RX.
  REG32(UART_CTRL(0)) =
      ((uart_ctrl_nco & UART_CTRL_NCO_MASK) << UART_CTRL_NCO_OFFSET) |
      (1 << UART_CTRL_TX) | (1 << UART_CTRL_RX);

  // Resets TX and RX FIFOs.
  uint32_t fifo_ctrl = REG32(UART_FIFO_CTRL(0));
  REG32(UART_FIFO_CTRL(0)) =
      fifo_ctrl | UART_FIFO_CTRL_RXRST | UART_FIFO_CTRL_TXRST;

  // Sets RX watermark to 1.
  //
  // This enables calls that block on a single byte at a time, like the one the
  // shell does when reading a line of input, to return immediately when that
  // byte is received.
  //
  // Note that this high watermark is only a threshold for when to be informed
  // that bytes have been received. The FIFO can still fill to its full capacity
  // (32) independent of how this is set.
  fifo_ctrl = REG32(UART_FIFO_CTRL(0));
  fifo_ctrl = fifo_ctrl & (~UART_FIFO_CTRL_RXILVL_MASK);
  fifo_ctrl = fifo_ctrl | (UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1
                           << UART_FIFO_CTRL_RXILVL_OFFSET);
  REG32(UART_FIFO_CTRL(0)) = fifo_ctrl;

  // Enables interrupts.
  REG32(UART_INTR_ENABLE(0)) = (1 << UART_INTR_COMMON_RX_WATERMARK);

  // TODO (mattharvey): Add tx_watermark_handle and make uart_tx have a buffer
  // and be interrupt-driven.

  rx_buf_end = rx_buf;
}

static int uart_get_rx_level() {
  return (REG32(UART_FIFO_STATUS(0)) &
          (UART_FIFO_STATUS_RXLVL_MASK << UART_FIFO_STATUS_RXLVL_OFFSET)) >>
         UART_FIFO_STATUS_RXLVL_OFFSET;
}

static int uart_rx_empty() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_RXEMPTY)) != 0;
}

static int uart_tx_ready() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_TXFULL)) == 0;
}

static char uart_getchar() {
  return REG32(UART_RDATA(0)) & UART_RDATA_RDATA_MASK;
}

static void uart_putchar(char c) { REG32(UART_WDATA(0)) = c; }

void uart_rx_update(size_t n) {
  char *dataport_cursor = (char *)rx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  seL4_Assert(n <= PAGE_SIZE);

  size_t num_read = 0;
  while (num_read < n) {
    while (rx_buf_end == rx_buf) {
      seL4_Assert(rx_semaphore_wait() == 0);
    }

    char *read_buf_cursor = rx_buf;
    while (num_read < n && read_buf_cursor < rx_buf_end) {
      *(dataport_cursor++) = *(read_buf_cursor++);
      ++num_read;
    }

    // Shifts remainder of rx_buf to the beginning.
    seL4_Assert(rx_mutex_lock() == 0);
    char *write_buf_cursor = rx_buf;
    while (read_buf_cursor < rx_buf_end) {
      *(write_buf_cursor++) = *(read_buf_cursor++);
    }
    rx_buf_end = write_buf_cursor;
    seL4_Assert(rx_mutex_unlock() == 0);
  }
}

void uart_tx_update(size_t n) {
  char *c = (char *)tx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  seL4_Assert(n <= PAGE_SIZE);

  for (size_t i = 0; i < n; ++i) {
    while (!uart_tx_ready()) {
      seL4_Yield();
    }
    uart_putchar(*c);
    ++c;
  }
}

void rx_watermark_handle(void) {
  size_t buf_remaining_size = sizeof(rx_buf) - (rx_buf_end - rx_buf);
  size_t num_read = 0;

  seL4_Assert(rx_mutex_lock() == 0);
  while (!uart_rx_empty() && num_read < buf_remaining_size) {
    *(rx_buf_end++) = uart_getchar();
    ++num_read;
  }
  seL4_Assert(rx_mutex_unlock() == 0);

  if (num_read > 0) {
    seL4_Assert(rx_semaphore_post() == 0);
  }

  // Clears INTR_STATE for rx_watermark.
  REG32(UART_INTR_STATE(0)) = (1 << UART_INTR_STATE_RX_WATERMARK);

  seL4_Assert(rx_watermark_acknowledge() == 0);
}
