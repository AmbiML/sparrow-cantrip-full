// 16550a UART driver
//
// Pared down from the xv6 RISC-V source (MIT license).
// https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/uart.c

#include <camkes.h>

#define UART0       (unsigned int)mem

// the UART control registers are memory-mapped
// at address UART0. this macro returns the
// address of one of the registers.
#define Reg(reg) ((volatile unsigned char *)(UART0 + reg))

// the UART control registers.
// some have different meanings for
// read vs write.
// see http://byterunner.com/16550.html
#define RHR 0                 // receive holding register (for input bytes)
#define THR 0                 // transmit holding register (for output bytes)
#define IER 1                 // interrupt enable register
#define IER_RX_ENABLE (1<<0)
#define IER_TX_ENABLE (1<<1)
#define FCR 2                 // FIFO control register
#define FCR_FIFO_ENABLE (1<<0)
#define FCR_FIFO_CLEAR (3<<1) // clear the content of the two FIFOs
#define ISR 2                 // interrupt status register
#define LCR 3                 // line control register
#define LCR_EIGHT_BITS (3<<0)
#define LCR_BAUD_LATCH (1<<7) // special mode to set baud rate
#define LSR 5                 // line status register
#define LSR_RX_READY (1<<0)   // input is waiting to be read from RHR
#define LSR_TX_IDLE (1<<5)    // THR can accept another character to send

#define ReadReg(reg) (*(Reg(reg)))
#define WriteReg(reg, v) (*(Reg(reg)) = (v))

void uart__init()
{
  // disable interrupts (UART from generating, not hart from dispatching)
  WriteReg(IER, 0x00);

  // special mode to set baud rate.
  WriteReg(LCR, LCR_BAUD_LATCH);

  // LSB for baud rate of 38.4K.
  WriteReg(0, 0x03);

  // MSB for baud rate of 38.4K.
  WriteReg(1, 0x00);

  // leave set-baud mode,
  // and set word length to 8 bits, no parity.
  WriteReg(LCR, LCR_EIGHT_BITS);

  // reset and enable FIFOs.
  WriteReg(FCR, FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

  // TODO (mattharvey): seL4 is not configured to dispatch UART interrupts to
  // this driver yet. Until that time, this driver spins to wait. The proper
  // thing will be to make a Rust embedded_hal implementation for Sparrow.
  //
  // enable transmit and receive interrupts.
  // WriteReg(IER, IER_TX_ENABLE | IER_RX_ENABLE);
}

static int uart_received()
{
  return ReadReg(LSR) & LSR_RX_READY;
}

static int is_transmit_empty() {
  return ReadReg(LSR) & LSR_TX_IDLE;
}

void uart_rx(size_t n) {
  char *c = (char*)rx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  for (size_t i = 0; i < n && i < PAGE_SIZE; ++i) {
    while (!uart_received());
    *c = ReadReg(RHR);
    ++c;
  }
}

void uart_tx(size_t n) {
  char *c = (char*)tx_dataport;
  // TODO(mattharvey): Error return value for n > PAGE_SIZE
  for (size_t i = 0; i < n && i < PAGE_SIZE; ++i) {
    while(!is_transmit_empty());
    WriteReg(THR, *c);
    ++c;
  }
}
