/*
 * Copyright 2021, Google LLC
 *
 * Demo component to show that concurrent control threads can be running.
 *
 * This component logs the first LOG_FIBONACCI_LIMIT Fibonacci numbers using the
 * LoggerInterface, waiting for INTERRUPTS_PER_WAIT interrupts between each
 * number. The messages are logged at level TRACE, which can be enabled by
 * issuing "loglevel trace" at the Cantrip prompt.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

// TODO(b/198360356): Remove this component when it is no longer needed for
// concurrency testing.

#include <camkes.h>
#include <sel4/config.h>
#include <stdint.h>

// How many Fibonacci numbers to write to the log.
#define LOG_FIBONACCI_LIMIT 80

#define INTERRUPTS_PER_VIRT_SEC (1000 / CONFIG_TIMER_TICK_MS)
#define INTERRUPTS_PER_WAIT (2 * INTERRUPTS_PER_VIRT_SEC)
#define LOGGER_INTERFACE_LOG_LEVEL 5

typedef uint64_t interrupt_count_t;

typedef struct {
  uint64_t f1;
  uint64_t f2;
  uint64_t n;
} fibonacci_state_t;

static void fibonacci_init(fibonacci_state_t *state) {
  state->f1 = 0;
  state->f2 = 1;
  state->n = 0;
}

static void fibonacci_increment(fibonacci_state_t *state) {
  uint64_t swap = state->f2;
  state->f2 = state->f1 + state->f2;
  state->f1 = swap;
  ++state->n;
}

static void wait(interrupt_count_t interrupt_count_to_wait,
                 interrupt_count_t *counter) {
  for (interrupt_count_t i = 0; i < interrupt_count_to_wait; ++i) {
    asm volatile("wfi");
    ++*counter;
  }
}

static float virtual_seconds(interrupt_count_t interrupt_count) {
  return interrupt_count / INTERRUPTS_PER_VIRT_SEC;
}

static uint64_t rdtime(void) {
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

static void fibonacci_log(const fibonacci_state_t *fibonacci_state,
                          interrupt_count_t interrupt_count) {
  char log_buf[128];
  snprintf(log_buf, sizeof(log_buf) / sizeof(char),
           "log_fibonacci:control: n == %llu; f == %llu; interrupt_count == "
           "%llu; rdtime == %llu; virt_sec ~= %.2f",
           fibonacci_state->n, fibonacci_state->f1, interrupt_count, rdtime(),
           virtual_seconds(interrupt_count));
  logger_log(LOGGER_INTERFACE_LOG_LEVEL, log_buf);
}

int run(void) {
  interrupt_count_t interrupt_count = 0;
  fibonacci_state_t fibonacci_state;
  fibonacci_init(&fibonacci_state);
  while (1) {
    wait(INTERRUPTS_PER_WAIT, &interrupt_count);
    if (fibonacci_state.n >= LOG_FIBONACCI_LIMIT) {
      fibonacci_init(&fibonacci_state);
    }
    fibonacci_log(&fibonacci_state, interrupt_count);
    fibonacci_increment(&fibonacci_state);
  }
}
