# sel4-sys

[![Crates.io](https://img.shields.io/crates/v/sel4-sys.svg?style=flat-square)](FIXME)

[Documentation](FIXME)

A Rust interface to the [seL4 kernel](https://sel4.systems). Raw syscall
bindings, kernel API, and data structure declarations.  Provides the
same interface that libsel4 does, with many C-isms removed.

NOTE: this module depends on the sel4-config crate to sync seL4 kernel
configuration. That crate and sel4-sys depend on the SEL4_DIR and
SEL4_OUT_DIR environment variables being set to the top of the seL4
kernel source and build directories.

## Current status

This is a work-in-progress. The crate has been heavily used on a riscv32
target for Rust code developed for CAmkES and for a new rootserver that
replaces capdl-loader-app. Other architectures have not been compiled and
are likely missing code in arch/*.rs.

Similarly seL4/CAmkES feature support is lightly tested. All features have
been compiled but, for example, only dynamic object allocation has been
(heavily) tested.

System calls use a Rust-friendly calling api (returning seL4_Result)
instead of return an seL4_Error.  Some kernel data structures have been
re-cast for use by Rust code and to minimize dependence on the seL4
kernel configuration. Work is ongoing to export all the seL4 data types
and definitions needed to build a complete system in Rust and to make data
types more Rust friendly (e.g. enums used for things like bitflags). Rust
code still depends on the sel4runtime library to handle getting to "main"
(or the equivalent CAmkES interface) but that can be distilled to <100
lines of code.

## Tracking seL4

Updating to a new version of seL4 should be straightforward. The scripts
in the tools directory are direct descendents of the seL4 tools scripts.
The intent is to merge the Rust support in tools/*.py into the seL4 code
base so that tracking seL4 requires limited work.

## Regression testing with sel4test

This crate can be used with sel4test for regression testing. There is
a wrapper crate for sel4-sys that generates C-callable api's to the
Rust code. Using this with the sel4test framework then gives you full
coverage of the system calls used by sel4test.

## Code structure

Code is broadly broken into 3 pieces:
1. Static arch-independent defniitions and interfaces: lib.rs
2. Static arch-dependent definitions and interfaces: arch/*.rs
3. Dynamically-generated definitions and interfaces, created by the
   tools/*.py scripts (TBD: merged into seL4 kernel)

#3 happens via cargo using a custom build.rs and the companion sel4-config
crate that syncs an seL4 kernel configuration. Various pieces of code are
glued together with Rust include directives. Target configuration keys off
the Rust target_arch and target_pointer_width.

TBD: dependent crates, optional features.

## TODO

- Fill in support for architectures other than riscv32
- Test all crate features (e.g. static object allocation)
- More complete coverage of seL4 api's
