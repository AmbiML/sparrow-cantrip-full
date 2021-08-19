/*
 * Copyright 2021, Google LLC
 *
 * A simple circular character buffer for use in CAmkES components.
 *
 * (thread-compatible but not thread-safe)
 *
 * It acts as a first-in-first-out queue of characters.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#include <stdbool.h>
#include <stddef.h>

#define CIRCULAR_BUFFER_CAPACITY 512

#ifndef WARN_UNUSED_RESELT
#define WARN_UNUSED_RESULT __attribute__((warn_unused_result))
#endif

typedef struct {
  char data[CIRCULAR_BUFFER_CAPACITY + 1];
  char *begin;
  char *end;
} circular_buffer;

// Call this exactly once before first use of a new circular_buffer.
void circular_buffer_init(circular_buffer *buf);

// Empties the buffer, discarding current data.
void circular_buffer_clear(circular_buffer *buf);

// Returns whether the buffer is empty.
bool circular_buffer_empty(const circular_buffer *buf) WARN_UNUSED_RESULT;

// Returns the number of chars currently in the buffer.
size_t circular_buffer_size(const circular_buffer *buf) WARN_UNUSED_RESULT;

// Returns the number of chars that can be written to the buffer before it will
// become full.
size_t circular_buffer_remaining(const circular_buffer *buf) WARN_UNUSED_RESULT;

// Removes the character least recently queued and returns it.
//
// If the buffer is empty, returns false and leaves the buffer *c unmodified.
bool circular_buffer_pop_front(circular_buffer *buf,
                               char *c) WARN_UNUSED_RESULT;

// Adds a character to the buffer.
//
// If the buffer is already full, returns false and leaves the buffer
// unmodified.
bool circular_buffer_push_back(circular_buffer *buf, char c) WARN_UNUSED_RESULT;
