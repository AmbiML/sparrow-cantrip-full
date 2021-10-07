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
#include "uart_driver_error.h"

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

#define LOCK(lockname) seL4_Assert(lockname##_lock() == 0)
#define UNLOCK(lockname) seL4_Assert(lockname##_unlock() == 0)
#define ASSERT_OR_RETURN(x)            \
  if (!(bool)(x)) {                    \
    return UARTDriver_AssertionFailed; \
  }

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

// Gets whether the receive FIFO empty status bit is set.
//
// Prefer this to FIFO_STATUS.RXLVL, which the simulation has sometimes reported
// as zero even when "not STATUS.RXEMPTY."
static bool rx_empty() { return REG(STATUS) & (1 << UART_STATUS_RXEMPTY); }

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

// Copies from tx_buf into the transmit FIFO.
//
// This stops when the transmit FIFO is full or when tx_buf is empty, whichever
// comes first.
static void fill_tx_fifo() {
  LOCK(tx_mutex);
  while (tx_fifo_level() < UART_FIFO_CAPACITY) {
    char c;
    if (!circular_buffer_pop_front(&tx_buf, &c)) {
      // The buffer is empty.
      break;
    }
    uart_putchar(c);
  }
  UNLOCK(tx_mutex);
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
  REG(INTR_ENABLE) =
      (BIT(UART_INTR_COMMON_TX_WATERMARK) | BIT(UART_INTR_COMMON_RX_WATERMARK) |
       BIT(UART_INTR_COMMON_TX_EMPTY));
}

// Implements Rust Read::read().
//
// Reads up to a given limit of bytes into the CAmkES rx_dataport, blocking
// until at least one byte is available.
int read_read(size_t limit) {
  if (limit > TX_RX_DATAPORT_CAPACITY) {
    return UARTDriver_OutOfDataportBounds;
  }
  char *cursor = (char *)rx_dataport;
  char *const cursor_begin = cursor;
  char *const cursor_limit = cursor_begin + limit;

  LOCK(rx_mutex);
  while (circular_buffer_empty(&rx_buf)) {
    UNLOCK(rx_mutex);
    seL4_Assert(rx_nonempty_semaphore_wait() == 0);
    LOCK(rx_mutex);
  }
  while (cursor < cursor_limit) {
    if (!circular_buffer_pop_front(&rx_buf, cursor)) {
      // The buffer is empty.
      seL4_Assert(rx_empty_semaphore_post() == 0);
      break;
    }
    ++cursor;
  }
  UNLOCK(rx_mutex);

  int num_read = cursor - cursor_begin;
  ASSERT_OR_RETURN(num_read > 0);
  return num_read;
}

// Implements Rust Write::write().
//
// Writes as many bytes from tx_dataport as the hardware will accept, but not
// more than the number available (specified by the argument). Returns the
// number of bytes written or a negative value if there is any error.
int write_write(size_t available) {
  if (available > TX_RX_DATAPORT_CAPACITY) {
    return UARTDriver_OutOfDataportBounds;
  }
  const char *cursor = (const char *)tx_dataport;
  const char *const cursor_begin = cursor;
  const char *const cursor_limit = cursor_begin + available;

  while (cursor < cursor_limit) {
    LOCK(tx_mutex);
    if (circular_buffer_remaining(&tx_buf) == 0) {
      break;
    }
    for (; cursor < cursor_limit; ++cursor) {
      if (!circular_buffer_push_back(&tx_buf, *cursor)) {
        // The buffer is full.
        break;
      }
    }
    UNLOCK(tx_mutex);
  }

  fill_tx_fifo();

  int num_written = cursor - cursor_begin;
  ASSERT_OR_RETURN(num_written > 0);
  return num_written;
}

// Implements Rust Write::flush().
//
// Drains tx_buf and TX_FIFO. Returns a negative value if there is any error.
int write_flush() {
  LOCK(tx_mutex);
  while (circular_buffer_remaining(&tx_buf)) {
    fill_tx_fifo();
  }
  UNLOCK(tx_mutex);
  return 0;
}

// Handles a tx_watermark interrupt.
//
// These happen when the transmit FIFO is half-empty. This refills the FIFO to
// prevent stalling, stopping early if tx_buf becomes empty, and then signals
// any tx_update that might be waiting for tx_buf to not be full.
void tx_watermark_handle(void) {
  fill_tx_fifo();

  // Clears INTR_STATE for tx_watermark. (INTR_STATE is write-1-to-clear.) No
  // similar check to the one in tx_empty_handle is necessary here, since
  // tx_empty will eventually assert and cause anything left in tx_buf to be
  // flushed out.
  REG(INTR_STATE) = BIT(UART_INTR_STATE_TX_WATERMARK);

  seL4_Assert(tx_watermark_acknowledge() == 0);
}

// Handles an rx_watermark interrupt.
//
// Reads any bytes currently pending in the receive FIFO into rx_buf, stopping
// early if rx_buf becomes full and then signals any call to rx_update that may
// be waiting on the condition that rx_buf not be empty.
void rx_watermark_handle(void) {
  LOCK(rx_mutex);
  while (!rx_empty()) {
    if (circular_buffer_remaining(&rx_buf) == 0) {
      // The buffer is full.
      //
      // We want to stay in this invocation of the interrupt handler until the
      // RX FIFO is empty, since the rx_watermark interrupt will not fire again
      // until the RX FIFO level crosses from 0 to 1. Therefore we unblock any
      // pending reads and wait for enough reads to consume all of rx_buf.
      seL4_Assert(rx_nonempty_semaphore_post() == 0);
      UNLOCK(rx_mutex);
      seL4_Assert(rx_empty_semaphore_wait() == 0);
      LOCK(rx_mutex);
      continue;
    }
    seL4_Assert(circular_buffer_push_back(&rx_buf, uart_getchar()));
  }
  seL4_Assert(rx_nonempty_semaphore_post() == 0);
  UNLOCK(rx_mutex);

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

  LOCK(tx_mutex);
  if (circular_buffer_empty(&tx_buf)) {
    // Clears INTR_STATE for tx_empty. (INTR_STATE is write-1-to-clear.) We
    // only do this if tx_buf is empty, since the TX FIFO might have become
    // empty in the time from fill_tx_fifo having sent the last character
    // until here. In that case, we want the interrupt to reassert.
    REG(INTR_STATE) = BIT(UART_INTR_STATE_TX_EMPTY);
  }
  UNLOCK(tx_mutex);
  seL4_Assert(tx_empty_acknowledge() == 0);
}
