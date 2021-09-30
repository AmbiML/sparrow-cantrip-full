# sel4-sys

[![Crates.io](https://img.shields.io/crates/v/sel4-sys.svg?style=flat-square)](https://crates.io/crates/sel4-sys)

[Documentation](https://doc.robigalia.org/sel4_sys)

A Rust interface to the [seL4 kernel](https://sel4.systems). Raw syscall
bindings, kernel API, and data structure declarations.  Provides the same
interface that libsel4 does, with a few C-isms reduced.

NOTE: be sure to `git submodule update --recursive --init` if you clone this
repository, as we pull in seL4 via a submodule.

## Updating to a new version of seL4

Updating to a new version of seL4 isn't hard, but it can be annoying.
First, cd into the `seL4` submodule, do a `git fetch`, and checkout the new
version you want to evaluate. Then, do a `cargo build`. At that point, you can
try running `cargo build`. It probably won't succeed, due to changes in API
and the Python tools.

To fix the Python tools, I use a command like:

    diff -u seL4/tools/bitfield_gen.py tools/bitfield_gen.py | pygmentize | less -R

I then carefully look at the diff to see if there are any meaningful
differences. One challenge when doing this is that a lot of some of the tools
has been ripped out, because it deals with topics Robigalia doesn't need to
care about (bitfield proofs, or declaration order, for example).

Once you have a successful `cargo build`, you're not done. It's likely that
the kernel added, removed, or otherwise changed various pieces of the ABI. In
particular, inspect `lib.rs` and update for any changes in the IPC buffer
(unlikely) or bootinfo (increasingly unlikely). Update `arch/x86_64.rs` etc
for any changes in the object types. Changes are usually easy to see by a cd
into seL4/libsel4 and a `git diff X.0.0..Y.0.0`.

As a quick smoketest, go to the `hello-world` repository and compile and run
it with the new kernel and `sel4-sys`.

After that, it's time to update the `sel4` crate and any other impacted user
components.

## Status

Mostly complete, though largely untested.

## TODO

- Add support iterating over the `seL4_BootInfoHeader`
- Add automated, comprehensive tests
- Formal verification that the code actually follows the implemented ABI.
