# Copyright 2022 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

RustAddLibrary(
  cantrip_ml_coordinator
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/../../components/MlCoordinator
  LIB_FILENAME libcantrip_ml_component.a
)

DeclareCAmkESComponent(
  MlCoordinator
  LIBS cantrip_ml_coordinator
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  cantrip_timer_service
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/../../components/TimerService
  LIB_FILENAME libcantrip_timer_component.a
)

DeclareCAmkESComponent(TimerService
  LIBS cantrip_timer_service
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

RustAddLibrary(
  mailbox_driver
  SOURCE_DIR ${CMAKE_CURRENT_LIST_DIR}/../../components/MailboxDriver
  LIB_FILENAME libmailbox_driver.a
)

DeclareCAmkESComponent(
  MailboxDriver
  LIBS mailbox_driver
  INCLUDES interfaces
  $ENV{OUT}/cantrip/components
)

DeclareCAmkESComponent(
  OpenTitanUARTDriver
  SOURCES
  ../../components/OpenTitanUARTDriver/src/driver.c
  ../../components/OpenTitanUARTDriver/src/circular_buffer.c
  INCLUDES
  ../../opentitan-gen/include
  ../../components/OpenTitanUARTDriver/include
  $ENV{OUT}/cantrip/components
)
