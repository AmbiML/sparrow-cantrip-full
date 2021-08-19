#!/bin/sh

# Quick and dirty script to test the pure C module the UART driver depends on.
# This can be run as needed with the development machine gcc.

TEST_BINARY=test_circular_buffer

cc -o $TEST_BINARY -Iinclude src/circular_buffer.c test/circular_buffer_test.c
./$TEST_BINARY
rm -f ./$TEST_BINARY
