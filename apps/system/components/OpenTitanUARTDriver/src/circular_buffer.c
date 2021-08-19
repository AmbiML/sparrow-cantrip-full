/*
 * Copyright 2021, Google LLC
 *
 * Implementation for circular_buffer.h
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#include "circular_buffer.h"

// Advances one of the begin/end pointers, wrapping around the end of the
// data array when necessary.
static void *circular_buffer_advance(circular_buffer *buf, char **p) {
  *p += 1;
  if (*p > buf->data + CIRCULAR_BUFFER_CAPACITY) {
    *p = buf->data;
  }
}

void circular_buffer_init(circular_buffer *buf) {
  circular_buffer_clear(buf);
}

void circular_buffer_clear(circular_buffer *buf) {
  buf->begin = buf->data;
  buf->end = buf->data;
}

bool circular_buffer_empty(const circular_buffer *buf) {
  return buf->begin == buf->end;
}

size_t circular_buffer_size(const circular_buffer *buf) {
  if (buf->end >= buf->begin) {
    // empty when end == begin
    return buf->end - buf->begin;
  } else {
    // full when begin == (end + 1)
    // cannot be empty in this branch
    return CIRCULAR_BUFFER_CAPACITY - (buf->begin - buf->end - 1);
  }
}

size_t circular_buffer_remaining(const circular_buffer *buf) {
  return CIRCULAR_BUFFER_CAPACITY - circular_buffer_size(buf);
}

bool circular_buffer_pop_front(circular_buffer *buf, char *c) {
  if(circular_buffer_empty(buf)) {
    return false;
  }
  *c = *buf->begin;
  circular_buffer_advance(buf, &buf->begin);
  return true;
}

bool circular_buffer_push_back(circular_buffer *buf, char c) {
  if(circular_buffer_remaining(buf) == 0) {
    return false;
  }
  *buf->end = c;
  circular_buffer_advance(buf, &buf->end);
  return true;
}
