// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Cantrip OS seL4 scheduling primitives

#![no_std]

/// Scheduling domains configured for seL4 TCBs.
///
/// Currently we have this setup as a single domain for all components, since we
/// don't want to waste 50% of our time waiting for a mostly idle partition.
///
/// TODO: Figure out how to more effectively use these domains of execution, and
/// how to prevent wasting time in an idle thread for a whole domain when no
/// TCBs are scheduled there. See also b/238811077.
///
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Domain {
    System = 0,
}
