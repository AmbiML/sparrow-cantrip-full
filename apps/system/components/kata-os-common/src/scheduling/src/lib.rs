//! Cantrip OS seL4 scheduling primitives

#![no_std]

/// Scheduling domains configured for seL4 TCBs.
///
/// Currently we have this setup as a pair of domains, one for the system
/// components and another for the third party application sandbox.
///
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Domain {
    System = 0,
    AppSandbox,
}
