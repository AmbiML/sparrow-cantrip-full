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
#include <utils/arith.h>

#include "circular_buffer.h"
#include "opentitan/uart.h"

// Referenced by macros in the generated file opentitan/uart.h.
#define UART0_BASE_ADDR (void *)mmio_region

// The TX/RX Fifo capacity mentioned in the programming guide.
#define UART_FIFO_CAPACITY 32ul

// This is the default in CAmkES 2 and the configurable default in CAmkES 3.
#define TX_RX_DATAPORT_CAPACITY PAGE_SIZE

// Frequency of the primary clock clk_i.
//
// TODO(mattharvey): OpenTitan actually specifies 24Mhz, but using that results
// in Renode reporting double the expected BaudRate.
//
// https://docs.opentitan.org/hw/ip/clkmgr/doc/
#define CLK_FIXED_FREQ_HZ (48ull * 1000 * 1000)

// Read/write access to a 32-bit register of UART0, using substrings of the
// #define names in opentitan/uart.h. (The literal 0 is the value of ##id##
// substitutions in uart.h.)
#define REG(name) *((volatile uint32_t *)(UART_##name(0)))

#define SHIFT_DOWN_AND_MASK(regval, regname, subfield) \
  ((regval >> UART_##regname##_##subfield##_OFFSET) &  \
   UART_##regname##_##subfield##_MASK)

#define MASK_AND_SHIFT_UP(value, regname, subfield) \
  ((value & UART_##regname##_##subfield##_MASK)     \
   << UART_##regname##_##subfield##_OFFSET)

// Driver-owned buffer to receive more than the FIFO size before the received
// data is consumed by rx_update.
static circular_buffer rx_buf;  // guarded by rx_mutex

// Driver-owned buffer to buffer more transmitted bytes than can fit in the
// transmit FIFO.
static circular_buffer tx_buf;  // guarded by tx_mutex

// Gets the number of unsent bytes in the TX FIFO from hardware MMIO.
static uint32_t tx_fifo_level() {
  return SHIFT_DOWN_AND_MASK(REG(FIFO_STATUS), FIFO_STATUS, TXLVL);
}

// Gets the number of pending bytes in the RX FIFO from hardware MMIO.
static uint32_t rx_fifo_level() {
  return SHIFT_DOWN_AND_MASK(REG(FIFO_STATUS), FIFO_STATUS, RXLVL);
}

// Reads one byte from the hardware read data register.
//
// Callers should first ensure the receive FIFO is not empty rather than rely on
// any particular magic value to indicate that.
static char uart_getchar() {
  return SHIFT_DOWN_AND_MASK(REG(RDATA), RDATA, RDATA);
}

// Writes one byte to the hardware write data register.
//
// The byte will be dropped if the transmit FIFO is empty.
static void uart_putchar(char c) {
  REG(WDATA) = MASK_AND_SHIFT_UP(c, WDATA, WDATA);
}

// Writes just enough of tx_buf to fill the transmit FIFO.
//
// This is called from all of tx_update, tx_watermark_handle, and
// tx_empty_handle. If tx_update has filled tx_buf to a size larger than the
// FIFO, interrupts will trigger and repeatedly until tx_buf is completely sent.
static void fill_tx_fifo() {
  // Caps the number of bytes sent to a fixed constant, since otherwise an
  // emulation reporting a very largo FIFO level could cause a long time to be
  // spent in an interrupt handler.
  uint32_t max_to_send = UART_FIFO_CAPACITY - tx_fifo_level();
  if (max_to_send > UART_FIFO_CAPACITY) {
    max_to_send = UART_FIFO_CAPACITY;
  }

  seL4_Assert(tx_mutex_lock() == 0);
  uint32_t capacity = circular_buffer_size(&tx_buf);
  for (uint32_t num_sent = 0; num_sent < max_to_send; ++num_sent) {
    char c;
    if (!circular_buffer_pop_front(&tx_buf, &c)) {
      // The buffer is empty.
      break;
    }
    uart_putchar(c);
  }
  if (circular_buffer_remaining(&tx_buf) > 0) {
    seL4_Assert(tx_semaphore_post() == 0);
  }
  seL4_Assert(tx_mutex_unlock() == 0);
}

// CAmkES initialization hook.
//
// Performs initial programming of the OpenTitan UART at mmio_region.
//
// In short, sets 115200bps, TX and RX on, and TX watermark to 1.
void pre_init() {
  // Clears the driver-owned buffers.
  circular_buffer_init(&tx_buf);
  circular_buffer_init(&rx_buf);

  // Computes NCO value corresponding to baud rate.
  // nco = 2^20 * baud / fclk  (assuming NCO width is 16-bit)
  seL4_CompileTimeAssert(UART_CTRL_NCO_MASK == 0xffff);
  uint64_t baud = 115200ull;
  uint64_t ctrl_nco = ((uint64_t)baud << 20) / CLK_FIXED_FREQ_HZ;
  seL4_Assert(ctrl_nco < 0xffff);

  // Sets baud rate and enables TX and RX.
  REG(CTRL) = MASK_AND_SHIFT_UP(ctrl_nco, CTRL, NCO) | BIT(UART_CTRL_TX) |
              BIT(UART_CTRL_RX);

  // Resets TX and RX FIFOs.
  uint32_t fifo_ctrl = REG(FIFO_CTRL);
  REG(FIFO_CTRL) =
      fifo_ctrl | BIT(UART_FIFO_CTRL_RXRST) | BIT(UART_FIFO_CTRL_TXRST);

  // Sets FIFO watermarks.
  fifo_ctrl = REG(FIFO_CTRL);
  // Clears old values of both watermarks.
  fifo_ctrl = fifo_ctrl &
              (~(UART_FIFO_CTRL_RXILVL_MASK << UART_FIFO_CTRL_RXILVL_OFFSET)) &
              (~(UART_FIFO_CTRL_TXILVL_MASK << UART_FIFO_CTRL_TXILVL_OFFSET));
  // RX watermark to 1.
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
  fifo_ctrl = fifo_ctrl | MASK_AND_SHIFT_UP(UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1,
                                            FIFO_CTRL, RXILVL);
  // TX watermark to 16 (half full).
  fifo_ctrl = fifo_ctrl | MASK_AND_SHIFT_UP(UART_FIFO_CTRL_TXILVL_VALUE_TXLVL16,
                                            FIFO_CTRL, TXILVL);
  REG(FIFO_CTRL) = fifo_ctrl;

  // Enables interrupts.
  REG(INTR_ENABLE) = (
      BIT(UART_INTR_COMMON_TX_WATERMARK) |
      BIT(UART_INTR_COMMON_RX_WATERMARK) | BIT(UART_INTR_COMMON_TX_EMPTY));
}

// Implements the update method of the CAmkES dataport_inf rx.
//
// Reads a given number of bytes from rx_buf into the CAmkES rx_dataport,
// blocking the RPC until the entire requested byte count has been read.
void rx_update(uint32_t num_to_read) {
  // TODO(mattharvey): Error return value for num_to_read >
  // TX_RX_DATAPORT_CAPACITY.
  seL4_Assert(num_to_read <= TX_RX_DATAPORT_CAPACITY);

  char *dataport_cursor = (char *)rx_dataport;
  char *const dataport_end = dataport_cursor + num_to_read;
  while (dataport_cursor < dataport_end) {
    seL4_Assert(rx_mutex_lock() == 0);
    while (circular_buffer_empty(&rx_buf)) {
      seL4_Assert(rx_mutex_unlock() == 0);
      seL4_Assert(rx_semaphore_wait() == 0);
      seL4_Assert(rx_mutex_lock() == 0);
    }
    for (; dataport_cursor < dataport_end; ++dataport_cursor) {
      if (!circular_buffer_pop_front(&rx_buf, dataport_cursor)) {
        // The buffer is empty.
        break;
      }
    }
    seL4_Assert(rx_mutex_unlock() == 0);
  }
}

// Implements the update method of the CAmkES dataport_inf tx.
//
// Writes the contents of the CAmkES tx_dataport to the UART, one at a time,
// blocking the RPC until the entire requested number of bytes has been written.
void tx_update(uint32_t num_valid_dataport_bytes) {
  // TODO(mattharvey): Error return value for num_valid_dataport_bytes >
  // TX_RX_DATAPORT_CAPACITY.
  seL4_Assert(num_valid_dataport_bytes <= TX_RX_DATAPORT_CAPACITY);

  const char *dataport_cursor = (const char *)tx_dataport;
  const char *const dataport_end = dataport_cursor + num_valid_dataport_bytes;
  while (dataport_cursor < dataport_end) {
    seL4_Assert(tx_mutex_lock() == 0);
    while (circular_buffer_remaining(&tx_buf) == 0) {
      seL4_Assert(tx_mutex_unlock() == 0);
      seL4_Assert(tx_semaphore_wait() == 0);
      seL4_Assert(tx_mutex_lock() == 0);
    }
    for (; dataport_cursor < dataport_end; ++dataport_cursor) {
      if (!circular_buffer_push_back(&tx_buf, *dataport_cursor)) {
        // The buffer is full.
        break;
      }
    }
    seL4_Assert(tx_mutex_unlock() == 0);
  }

  if (tx_fifo_level() == 0) {
    // If the FIFO is already empty, there is no interrupt coming, so we trigger
    // the first transmission manually.
    fill_tx_fifo();
  }
}

// Handles a tx_watermark interrupt.
//
// These happen when the transmit FIFO is half-empty. This refills the FIFO to
// prevent stalling, stopping early if tx_buf becomes empty, and then signals
// any tx_update that might be waiting for tx_buf to not be full.
void tx_watermark_handle(void) {
  fill_tx_fifo();

  // Clears INTR_STATE for tx_watermark. (INTR_STATE is write-1-to-clear.)
  REG(INTR_STATE) = BIT(UART_INTR_STATE_TX_WATERMARK);

  seL4_Assert(tx_watermark_acknowledge() == 0);
}

// Handles an rx_watermark interrupt.
//
// Reads any bytes currently pending in the receive FIFO into rx_buf, stopping
// early if rx_buf becomes full and then signals any call to rx_update that may
// be waiting on the condition that rx_buf not be empty.
void rx_watermark_handle(void) {
  // Set a constant cap on the number of bytes read to ensure the interrupt
  // handler returns promptly, even on emulations that report an unusually large
  // FIFO level.
  uint32_t num_to_read = rx_fifo_level();
  if (num_to_read > UART_FIFO_CAPACITY) {
    num_to_read = UART_FIFO_CAPACITY;
  }

  uint32_t num_read = 0;
  seL4_Assert(rx_mutex_lock() == 0);
  while (num_read < num_to_read) {
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
  REG(INTR_STATE) = BIT(UART_INTR_STATE_RX_WATERMARK);

  seL4_Assert(rx_watermark_acknowledge() == 0);
}

// Handles a tx_empty interrupt.
//
// This copies tx_buf into the hardware transmit FIFO, stopping early if tx_buf
// becomes empty, and then signals any tx_update that might be waiting for
// tx_buf to not be full.
void tx_empty_handle(void) {
  fill_tx_fifo();

  // Clears INTR_STATE for tx_empty. (INTR_STATE is write-1-to-clear.)
  REG(INTR_STATE) = BIT(UART_INTR_STATE_TX_EMPTY);

  seL4_Assert(tx_empty_acknowledge() == 0);
}
