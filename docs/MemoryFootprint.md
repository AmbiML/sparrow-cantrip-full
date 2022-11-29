
## Memory footprint

The initial Sparrow target platform was intended to have 4MiB of
memory. A production build of the included services fit in <2MiB
of memory but due to the overhead of CAmkES and the rootserver
boostrap mechanism actually require ~2x that to reach a running
state. The kmem.sh script can be used to analyze memory use. These
services should easily fit in <1MiB but due to CAmkES overhead
(e.g. per-thread cost) are significantly bloated. We use kmem and the
[bloaty tool](https://github.com/google/bloaty) to evaluate memory use.
Reducing memory use to <1MiB likely requires replacing CAmkES by a native
Rust framework like [embassy](https://github.com/embassy-rs/embassy).

We also expect that replacing CAmkES C-based templates by native Rust code
will significantly reduce memory use (as well as improve robustness by
extending the scope of the borrow checker). The RPC mechanism used for
communication between applications and the *SDKRuntime* is a prototype
of a native Rust implementation that demonstrates this.

### [Next Section: CantripOS capDL rootserver application](CantripRootserver.md)
