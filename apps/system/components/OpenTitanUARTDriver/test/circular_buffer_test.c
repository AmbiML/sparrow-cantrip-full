/*
 * Copyright 2021, Google LLC
 *
 * Tests for circular_buffer.
 *
 * Run these with OpenTitanUARTDriver/test.sh.
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#include "circular_buffer.h"

#include <stddef.h>
#include <stdio.h>

// These are currently quick and dirty tests with a minimal "framework." When
// adding new tests, be sure to add a TEST(...); declaration to main().

// TODO(mattharvey): Determine how to do unit testing within the cantrip build
// system and port this to that framework.

#define TEST(X)         \
  printf("%s\n", #X);   \
  if ((X)()) {          \
    printf("\tpass\n"); \
  }

#define ASSERT(X)                                                       \
  if (!(X)) {                                                           \
    printf("\tfailed assertion: (%s:%d) %s\n", __FILE__, __LINE__, #X); \
    return false;                                                       \
  }

static void fill_with_x(circular_buffer* buf) {
  circular_buffer_init(buf);
  const char c = 'x';
  for (size_t i = 0; i < CIRCULAR_BUFFER_CAPACITY; ++i) {
    const bool success = circular_buffer_push_back(buf, c);
    (void)success;
  }
}

bool test_size_of_empty() {
  circular_buffer buf;
  circular_buffer_init(&buf);

  ASSERT(circular_buffer_empty(&buf));
  ASSERT(circular_buffer_remaining(&buf) == CIRCULAR_BUFFER_CAPACITY);

  return true;
}

bool test_double_push_double_pop() {
  circular_buffer buf;
  circular_buffer_init(&buf);

  char push = 'a';
  ASSERT(circular_buffer_push_back(&buf, push));
  push = 'b';
  ASSERT(circular_buffer_push_back(&buf, push));

  char pop;
  ASSERT(circular_buffer_pop_front(&buf, &pop));
  ASSERT(pop == 'a');
  ASSERT(circular_buffer_pop_front(&buf, &pop));
  ASSERT(pop == 'b');

  return true;
}

bool test_size_of_full() {
  circular_buffer buf;
  fill_with_x(&buf);

  ASSERT(circular_buffer_remaining(&buf) == 0);
  ASSERT(circular_buffer_size(&buf) == CIRCULAR_BUFFER_CAPACITY);

  return true;
}

bool test_push_full() {
  circular_buffer buf;
  fill_with_x(&buf);

  ASSERT(circular_buffer_push_back(&buf, 'x') == false);

  return true;
}

bool test_pop_empty() {
  circular_buffer buf;
  circular_buffer_init(&buf);

  char pop;
  ASSERT(circular_buffer_pop_front(&buf, &pop) == false);

  return true;
}

bool test_clear_full() {
  circular_buffer buf;
  fill_with_x(&buf);

  circular_buffer_clear(&buf);

  ASSERT(circular_buffer_empty(&buf));
  ASSERT(circular_buffer_remaining(&buf) == CIRCULAR_BUFFER_CAPACITY);

  return true;
}

bool test_rotating_push_pop() {
  // We'll push and pop a single character enough to wrap around a few times.
  const char push = 'x';
  char pop;
  circular_buffer buf;
  circular_buffer_init(&buf);

  for (size_t i = 0; i < 10 * CIRCULAR_BUFFER_CAPACITY; ++i) {
    pop = 0;
    ASSERT(circular_buffer_push_back(&buf, push));
    ASSERT(circular_buffer_pop_front(&buf, &pop));
    ASSERT(pop == push);
  }

  ASSERT(circular_buffer_empty(&buf));

  return true;
}

int main(int argc, char** argv) {
  TEST(test_size_of_empty);
  TEST(test_double_push_double_pop);
  TEST(test_size_of_full);
  TEST(test_push_full);
  TEST(test_pop_empty);
  TEST(test_clear_full);
  TEST(test_rotating_push_pop);
}
