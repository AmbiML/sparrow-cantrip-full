/*
 * Copyright 2021, Google LLC
 *
 * A programming guide for the hardware can be found at
 * https://docs.opentitan.org/hw/ip/uart/doc/
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <camkes.h>
#include <sel4/syscalls.h>
#include <stdbool.h>
#include <stdint.h>

#include "circular_buffer.h"
#include "opentitan/uart.h"

// Referenced by macros in the generated file opentitan/uart.h.
#define UART0_BASE_ADDR (void *)mmio_region

// This is the default in CAmkES 2 and the configurable default in CAmkES 3.
#define TX_RX_DATAPORT_CAPACITY PAGE_SIZE

// Frequency of the primary clock clk_i.
//
// TODO(mattharvey): OpenTitan actually specifies 24Mhz, but using that results
// in Renode reporting double the expected BaudRate.
//
// https://docs.opentitan.org/hw/ip/clkmgr/doc/
#define CLK_FIXED_FREQ_HZ (48ull * 1000 * 1000)

// Read/write access to a single 32-bit register.
#define REG32(addr) *((volatile uint32_t *)(addr))

// Driver-owned buffer to receive more than the FIFO size before the received
// data is consumed by rx_update.
static circular_buffer rx_buf;  // guarded by rx_mutex

// Returns whether the hardware says the receive FIFO is empty.
static bool rx_empty() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_RXEMPTY)) != 0;
}

// Returns whether the hardware is ready to take another byte for transmit.
static bool tx_ready() {
  return (REG32(UART_STATUS(0)) & (1 << UART_STATUS_TXFULL)) == 0;
}

// Reads one byte from the hardware read data register.
//
// Callers should first ensure the receive FIFO is not empty rather than rely on
// any particular magic value to indicate that.
static char uart_getchar() {
  return REG32(UART_RDATA(0)) & UART_RDATA_RDATA_MASK;
}

// Writes one byte to the hardware write data register.
//
// The byte will be dropped if the transmit FIFO is empty.
static void uart_putchar(char c) { REG32(UART_WDATA(0)) = c; }

// CAmkES initialization hook.
//
// Performs initial programming of the OpenTitan UART at mmio_region.
//
// In short, sets 115200bps, TX and RX on, and TX watermark to 1.
void pre_init() {
  // Computes NCO value corresponding to baud rate.
  // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
  seL4_CompileTimeAssert(UART_CTRL_NCO_MASK == 0xffff);
  uint64_t baud = 115200ull;
  uint64_t ctrl_nco = ((uint64_t)baud << 20) / CLK_FIXED_FREQ_HZ;
  seL4_Assert(ctrl_nco < 0xffff);

  // Sets baud rate and enables TX and RX.
  REG32(UART_CTRL(0)) =
      ((ctrl_nco & UART_CTRL_NCO_MASK) << UART_CTRL_NCO_OFFSET) |
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
  //
  // Although a higher watermark in combination with rx_timeout might be
  // preferable, Renode simulation does not yet support the rx_timeout
  // interrupt.
  fifo_ctrl = REG32(UART_FIFO_CTRL(0));
  fifo_ctrl = fifo_ctrl & (~UART_FIFO_CTRL_RXILVL_MASK);
  fifo_ctrl = fifo_ctrl | (UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1
                           << UART_FIFO_CTRL_RXILVL_OFFSET);
  REG32(UART_FIFO_CTRL(0)) = fifo_ctrl;

  // Enables interrupts.
  REG32(UART_INTR_ENABLE(0)) = (1 << UART_INTR_COMMON_RX_WATERMARK);

  // TODO (mattharvey): Add tx_buf and tx_watermark_handle so that tx_update
  // calls only need to block if tx_buf is full.

  circular_buffer_clear(&rx_buf);
}

// Implements the update method of the CAmkES dataport_inf rx.
//
// Reads a given number of bytes from rx_buf into the CAmkES rx_dataport,
// blocking the RPC until the entire requested byte count has been read.
void rx_update(size_t num_to_read) {
  char *dataport_cursor = (char *)rx_dataport;
  // TODO(mattharvey): Error return value for num_to_read >
  // TX_RX_DATAPORT_CAPACITY.
  seL4_Assert(num_to_read <= TX_RX_DATAPORT_CAPACITY);

  size_t num_read = 0;
  while (num_read < num_to_read) {
    while (circular_buffer_empty(&rx_buf)) {
      seL4_Assert(rx_semaphore_wait() == 0);
    }
    seL4_Assert(rx_mutex_lock() == 0);
    while (num_read < num_to_read && !circular_buffer_empty(&rx_buf)) {
      char c;
      if (!circular_buffer_pop_front(&rx_buf, &c)) {
        // The buffer is empty.
        break;
      }
      *(dataport_cursor++) = c;
      ++num_read;
    }
    seL4_Assert(rx_mutex_unlock() == 0);
  }
}

// Implements the update method of the CAmkES dataport_inf tx.
//
// Writes the contents of the CAmkES tx_dataport to the UART, one at a time,
// blocking the RPC until the entire requested number of bytes has been written.
void tx_update(size_t num_valid_dataport_bytes) {
  char *c = (char *)tx_dataport;
  // TODO(mattharvey): Error return value for num_valid_dataport_bytes >
  // TX_RX_DATAPORT_CAPACITY.
  seL4_Assert(num_valid_dataport_bytes <= TX_RX_DATAPORT_CAPACITY);

  for (size_t i = 0; i < num_valid_dataport_bytes; ++i) {
    while (!tx_ready()) {
      seL4_Yield();
    }
    uart_putchar(*c);
    ++c;
  }
}

// Handles an rx_watermark interrupt.
//
// Reads from the receive FIFO into rx_buf until rx_buf is full or the FIFO is
// empty, whichever happens first, and then signals any call to tx_update that
// may be waiting on the condition that rx_buf not be empty.
void rx_watermark_handle(void) {
  size_t num_read = 0;

  seL4_Assert(rx_mutex_lock() == 0);
  size_t buf_remaining_size = circular_buffer_remaining(&rx_buf);
  while (!rx_empty() && num_read < buf_remaining_size) {
    if (!circular_buffer_push_back(&rx_buf, uart_getchar())) {
      // The buffer is full.
      break;
    }
    ++num_read;
  }
  seL4_Assert(rx_mutex_unlock() == 0);

  if (num_read > 0) {
    seL4_Assert(rx_semaphore_post() == 0);
  }

  // Clears INTR_STATE for rx_watermark. (INTR_STATE is write-1-to-clear.)
  REG32(UART_INTR_STATE(0)) = (1 << UART_INTR_STATE_RX_WATERMARK);

  seL4_Assert(rx_watermark_acknowledge() == 0);
}
