
## Memory footprint

A release build for Sparrow fits in ~1MiB of memory and boots to
a running system in 4MiB.
The boostrap mechanism (using capDL and the rootserver) actually
requires ~2x the idle memory footprint to reach a running state
(due to the overhead of the rootserver instantiating the system).
The kmem.sh script can be used to analyze memory use.
We use kmem and the
[bloaty tool](https://github.com/google/bloaty) to evaluate memory use.
For example,

```shell
$ kmem.sh
debug_console                                 130 KiB              133200
mailbox_driver                                 66 KiB               68416
memory_manager                                119 KiB              122416
...
uart_driver                                    70 KiB               72528
...
GRAND TOTAL                                  1163 KiB             1190976
```

Note that kmem.sh accounts for seL4 kernel objects for the Sparrow platform
but lacks the necessary object sizes for other platforms.
kmem output tries to aggregate resources by CAmkES component but for shared
resources (e.g. mapped pages used for RPC) may appear as separate items; e.g.

```shelll
multi_logger                                   32 KiB               32800
```

Memory use in CantripOS was lowered by replacing the C-based CAmkES' runtime by
a native Rust framework.
The CAmkES tool that processes an assembly specification and does resource
allocation generates minimal code (almost entirely definitions).
The cantrip-os-camkes crate implements all runtime functionality and is
designed to be used directly in case the generated template code is
not suitable.
This can be used, for example, to prototype new RPC mechanisms before writing
any template code.
Overall the Rust template support improves performance and robustness by
extending the scope of the borrow checker and enabling the optimizer
to work across the entirety of a thread's implementation.

Another important difference between CantripOS CAmkES use and the legacy code
is no Interface Definitions are used (anything specified is ignored).
Instead one writes pure Rust that handles parameter marshaling and logistics.
Two RPC implementations are provided: `rpc_basic` and `rpc_shared`.
This allows for direct marshaling & unmarshaling of parameters into the IPC
buffer which reduces memory use and memory copies.
The long-term goal for RPC support is to provide Rust derive macros that can
be used to annotate data structures with automatic generation of boilerplate code.

The other notable change in CantripOS is more fine-grained control for when
CAmkES threads are created.
This has two forms: shared IRQ's and explicitly disabling interface threads.
Normally each IRQ has a dedicated thread that services it.
When there are low-priority IRQ's aggregating them so they are directed to
one thread one can eliminate the static per-thread overhead (stack, ipc_buffer,
kernel resources).
For example, in the Sparrow system.camkes file the mailbox driver services multiple
IRQ's in a single thread:

```
connection cantripIRQ mailbox_driver_irq(
    from mailbox_hardware.wtirq,
    from mailbox_hardware.eirq,
    to mailbox_driver.irq)
```

and, in fact, the MailboxDriver's control thread is re-purposed to do this by
specifying:

```
consumes Interrupt irq;
attribute int irq_has_thread = false;
```

in MailboxDriver.camkes and supplying a run method that processes the IRQ's:

```
fn run() {
    // NB: do not handle rtirq, it blocks waiting for the api thread
    shared_irq_loop!(
        irq,
        wtirq => WtirqInterfaceThread::handler,
        eirq => EirqInterfaceThread::handler
    );
}
```

### [Next Section: CantripOS capDL rootserver application](CantripRootserver.md)
