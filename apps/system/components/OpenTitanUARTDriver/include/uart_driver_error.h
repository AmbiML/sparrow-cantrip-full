/*
 * Copyright 2021, Google LLC
 *
 * Error codes for the OpenTitanUARTDriver.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

// Return codes for errors on read() and write().
//
// Normally these functions return the number of bytes actually read or written,
// with 0 indicating the end of the stream. If something goes wrong, the
// functions will return one of these negative values.
typedef enum UARTDriverError {
  UARTDriver_AssertionFailed = -1,
  UARTDriver_OutOfDataportBounds = -2,
} uart_driver_error_t;
