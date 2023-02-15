
## Memory footprint

A release build for Sparrow fits in ~1.5MiB of memory and boots to
a running system in 4MiB.
The boostrap mechanism (using capDL and the rootserver) actually
requires ~2x the idle memory footprint to reach a running state
(due to the overhead of the rootserver instantiating the system).
The kmem.sh script can be used to analyze memory use. System
services should easily fit in 1MiB but due to CAmkES overhead
(e.g. per-thread cost) are significantly bloated. We use kmem and the
[bloaty tool](https://github.com/google/bloaty) to evaluate memory use.

To reduce memory use to <1MiB we are replacing the
CAmkES' runtime by a native Rust framework.
This should also improve performance and robustness by
extending the scope of the borrow checker and enabling the optimizer
to work across C <> Rust runtime boundaries that are a byproduct
of the CAmkES C-based implementation.
The RPC mechanism used for
communication between applications and the *SDKRuntime* is a prototype
of a native Rust implementation that demonstrates where we're headed.

### [Next Section: CantripOS capDL rootserver application](CantripRootserver.md)
