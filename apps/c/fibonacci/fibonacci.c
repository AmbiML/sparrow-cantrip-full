/*
 * Copyright 2021, Google LLC
 *
 * Demo to show that concurrent applications can be running.
 *
 * This program prints the first LOG_FIBONACCI_LIMIT Fibonacci numbers
 * to the console, waiting for INTERRUPTS_PER_WAIT interrupts between each
 * number.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#include <cantrip.h>
#include <stdint.h>

// How many Fibonacci numbers to write to the log.
#define LOG_FIBONACCI_LIMIT 80

#define CONFIG_TIMER_TICK_MS 5
#define INTERRUPTS_PER_VIRT_SEC (1000 / CONFIG_TIMER_TICK_MS)
#define INTERRUPTS_PER_WAIT (1 * INTERRUPTS_PER_VIRT_SEC)

typedef uint64_t interrupt_count_t;

typedef struct {
  uint64_t f1;
  uint64_t f2;
  uint64_t n;
} fibonacci_state_t;

void fibonacci_init(fibonacci_state_t *state) {
  state->f1 = 0;
  state->f2 = 1;
  state->n = 0;
}

void fibonacci_increment(fibonacci_state_t *state) {
  uint64_t swap = state->f2;
  state->f2 = state->f1 + state->f2;
  state->f1 = swap;
  ++state->n;
}

void wait(interrupt_count_t interrupt_count_to_wait,
          interrupt_count_t *counter) {
  for (interrupt_count_t i = 0; i < interrupt_count_to_wait; ++i) {
    asm volatile("wfi");
    ++*counter;
  }
}

float virtual_seconds(interrupt_count_t interrupt_count) {
  return interrupt_count / INTERRUPTS_PER_VIRT_SEC;
}

uint64_t rdtime(void) {
  uint32_t upper, lower, upper_reread;
  while (1) {
    asm volatile(
        "rdtimeh %0\n"
        "rdtime  %1\n"
        "rdtimeh %2\n"
        : "=r"(upper), "=r"(lower), "=r"(upper_reread));
    if (upper_reread == upper) {
      return ((uint64_t)upper << 32) | lower;
    }
  }
}

void fibonacci_log(int pid, const fibonacci_state_t *fibonacci_state,
                   interrupt_count_t interrupt_count) {
// TODO(sleffler): bring in snprintf
#if 0
  char log_buf[128];
  snprintf(log_buf, sizeof(log_buf) / sizeof(char),
           "\nfibonacci: n == %llu; f == %llu; interrupt_count == "
           "%llu; rdtime == %llu; virt_sec ~= %.2f\n",
           fibonacci_state->n, fibonacci_state->f1, interrupt_count, rdtime(),
           virtual_seconds(interrupt_count));
  debug_printf(log_buf);
#else
  debug_printf(
      "[%d]: "
      "n == %d; "
      "f == %x; "
      "interrupt_count == %d; "
      "rdtime == %d; "
      "virt_sec ~= %d\n",
      pid, (uint32_t)fibonacci_state->n, (uint32_t)fibonacci_state->f1,
      (uint32_t)interrupt_count, (uint32_t)rdtime(),
      (uint32_t)virtual_seconds(interrupt_count));
#endif
}

int main(int pid, int a1, int a2, int a3) {
  interrupt_count_t interrupt_count = 0;
  fibonacci_state_t fibonacci_state;
  fibonacci_init(&fibonacci_state);
  debug_printf("\nFibonacci: pid %d\n", pid);
  while (1) {
    wait(INTERRUPTS_PER_WAIT, &interrupt_count);
    if (fibonacci_state.n >= LOG_FIBONACCI_LIMIT) {
      fibonacci_init(&fibonacci_state);
    }
    fibonacci_log(pid, &fibonacci_state, interrupt_count);
    fibonacci_increment(&fibonacci_state);
  }
}
