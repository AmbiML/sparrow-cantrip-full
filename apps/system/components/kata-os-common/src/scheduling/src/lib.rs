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
