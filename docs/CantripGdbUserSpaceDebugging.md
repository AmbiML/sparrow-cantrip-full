# How to Use GDB with Renode to Debug seL4 Threads with Symbols

## High-Level Usage

The method for debugging seL4 threads with Renode is relatively simple, and
involves the usual suspects of `m debug-simulation` (or `m simulate-debug`) and `kgdb.sh`, but the
interface through GDB has changed slightly. In general, a debugging session now
looks like the following:

On one terminal, start an instance of Renode:
```bash
source build/setup.sh
m debug-simulation
```

[Note `m debug-simulation` gives GDB
control before anything runs while `m simulate-debug` lets Renode run
until `kgdb.sh` is invoked and contacts GDB. The former is necessary to
debug issues in the boot process (bootstrap, kernel, rootserver).]

Once Renode has started and starts outputting green debug logs, in another
terminal start up GDB with:
```bash
kgdb.sh
```

At this point, GDB should connect to the running Renode instance started by
`debug-simulation`, and you'll see something equivalent to the following (output
is not exact -- symbol reading may differ):

```
ghost:~/$ kgdb.sh
Source directories searched: /usr/local/google/home/sleffler/sparrow/sw/tock:$cdir:$cwd
Reading symbols from out/sparrow_boot_rom/multihart_boot_rom/multihart_boot_rom.elf...
add symbol table from file "out/sparrow_boot_rom/multihart_boot_rom/multihart_boot_rom.elf"
Reading symbols from out/sparrow_boot_rom/multihart_boot_rom/multihart_boot_rom.elf...
add symbol table from file "out/matcha/riscv32imc-unknown-none-elf/debug/matcha_platform"
Reading symbols from out/matcha/riscv32imc-unknown-none-elf/debug/matcha_platform...
add symbol table from file "out/matcha/riscv32imc-unknown-none-elf/debug/matcha_app"
Reading symbols from out/matcha/riscv32imc-unknown-none-elf/debug/matcha_app...
Remote debugging using localhost:3333
warning: multi-threaded target stopped without sending a thread-id, using first non-exited thread
0x00008090 in _reset_start ()
Symbol autoswitching is now disabled
(gdb)
```

At this point, GDB has stopped the simulator, giving you a chance to set
breakpoints and examine the system.

`kgdb.sh` will load symbols on the seL4 side, based on their
thread / CAmkES component, so there is no need to manually find the associated symbol file
and load it with the correct offsets. It may be necessary to switch symbol
tables, which can easily be done using `sel4 switch-symbols <component>`, which
will automatically load the symbols for CAmkES `<component>` with the appropriate offsets
(a `<component>` is the lower-case version of the CAmkES name, e.g. the CAmkES component
`DebugConsole` translates to `debug_console`.)

In this environment, there are two sets of breakpoints that can be used:
standard GDB `break`-style breakpoints, and renode-side `sel4 break`
breakpoints. The former work within the currently executing thread and can be
used for "local" breaks inside of a thread, whereas the latter can be set in any
thread that was previously discovered by the Renode extension.

Once a breakpoint is setup using `sel4 break` one can just
`continue`. For example, consider this debugging session:

```
<assuming cpu1 is halted and GDB is waiting at a prompt>

(gdb) sel4 wait-for-thread rootserver

Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
[Switching to Thread 2]
0xff814802 in ?? ()
(gdb) # Switch to the 'rootserver' thread's symbol table and set a breakpoint on main
(gdb) sel4 switch-symbols rootserver
Reading symbols from out/cantrip/riscv32-unknown-elf/debug/capdl-loader...
(gdb) sel4 break rootserver main
(gdb) c
Continuing.

Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
cantrip_os_rootserver::main () at src/main.rs:164
164     pub fn main() {
(gdb)
(gdb) # As we are in capdl-loader thread and proper symbols are loaded, all commands
(gdb) # like break work as expected.
(gdb) b init_system
Breakpoint 1 at 0x4c632: file /usr/local/google/home/sleffler/sparrow/cantrip/projects/cantrip/apps/system/components/cantrip-os-common/src/model/mod.rs, line 200.
(gdb) c
Continuing.

Thread 2 "matcha.cpu1" hit Breakpoint 1, model::CantripOsModel::init_system (self=0x8e3cc8)
    at /usr/local/google/home/sleffler/sparrow/cantrip/projects/cantrip/apps/system/components/cantrip-os-common/src/model/mod.rs:200
200             self.init_copy_region();
```

Note that symbol tables are per-component but breakpoints are
per-thread. The seL4 GDB extentions assume all threads in a component
share the same VSpace and hence the same symbol table. Beware also
that when symbol names are ambiguous you can specify fully-qualified
names; e.g. above `main` was expanded to `cantrip_os_rootserver::main` and
`init_system` to `model::CantripOsModel::init_system`. Several well-known
special symbols are supported: `kernel` for the seL4 Kernel, `user`
for any thread running in user space, and `rootserver` for the first
seL4 thread that runs after boot.

## Temporary Breakpoints

In addition to regular GDB breakpoints the sel4 extentions support temporary
breakpoints. The syntax is `sel4 tbreak <threadname> [<symbol>]`. If `<symbol>`
is not specified GDB will stop on the next entry to `<threadname>`. This is
especially useful for stopping the next time you enter user space:

```
(gdb) sel4 thread
kernel
(gdb) sel4 tbreak user
(gdb) c
Continuing.
Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
[Switching to Thread 2]
0xff80311c in ?? ()
```

or the kernel:

```
(gdb) sel4 tbreak kernel
(gdb) sel4 list-breakpoints
--------------------------
|Thread|Address|Temporary|
--------------------------
|kernel|any    |True     |
--------------------------
(gdb) c
Continuing.

Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
0xff803000 in ?? ()
(gdb)
```

## Theory of Operation

The kgdb.sh / Renode symbol lookup mechanism is
composed of two parts: a custom Renode extension in
`sim/config/sparrow_infrastructure/seL4Extensions.cs` and a custom GDB
extension in `sim/renode/tools/sel4_extensions/gdbscript.py`. `kgdb.sh`
is a simple bash script that starts GDB and connects to the appropriate
remote with the extension loaded.

The Renode extension tracks the VSpace setup for each thread and
translates virtual addresses to physical addresses. Without this work
symbols will be (almost always) incorrectly resolved because every VSpace
memory range starts at 0.

In addition, the extension provides commands to Renode to add debugging hooks to
an execution, all prefixed with the subcommand `seL4` (case matters at the
Renode prompt!). In our simulation config, Renode is configured to start the
seL4 extension on cpu1 using the `cpu1 CreateseL4` command.

The GDB extension provides a matching set of wrappers via the `sel4` command
prefix to examine the simulator state and take a look at threads in the system.

Both extensions provide built in help at their respective command lines, but
suffice it to say, the GDB extension is essentially just an automation mechanism
to load in symbols for the current thread context from disk when the Renode
extension detects a context switch.

During debugging, most symbols will not be loaded in GDB at the start, so it can
be helpful to use `sel4 wait-for-thread <threadname>` to tell Renode to break
execution upon entry of the given thread. When a break occurs the appropriate
symbols may be loaded with the `sel4 switch-symbols <threadname>` command or,
if `sel4 symbol-autoswitching true` is set then Renode will automatically switch
symbol tables based on the current thread. Symbol-autoswitching is disabled by
default because GDB does not have a reliable way to synchronize the
cmdline with the runtime so repeatedly switching between threads can leave
GDB in a bad state where it will crash on an assert.

To set a breakpoint for a thread that is not the current context, the thread
needs to be entered at least once. After that symbols in that thread can be
resolved so the `sel4 break <threadname> <symbol>` command works. Beware that
if `<symbol>` is wrong (e.g. mis-typed) the error diagnostic will go to the
Renode console, not to the GDB cmdline. Note also that using regular GDB
breakpoints will generate complaints about not being able to set the breakpoints
when switching to a different thread. Best to use `sel4 break` in seL4 user
threads and limit GDB `break` to contexts like the kernel or multihart_boot_rom.

## Debugging CantripOS applications

CantripOS supports dynamically loading application programs.
An application is much like a system component and GDB can be used similarl
to the description above except for loading symbols. Unlike a system component
there is no well-known mapping from the thread name to the symbol file
so you must manually load the symbols with the GDB add-symbol-file command.
For example, first you need to get Renode to the point where it knows about
the application's thread. This is done by first installing the application:

```
CANTRIP> install keyval.app
Collected 445892 bytes of data, crc32 a1275adf
Application "keyval" installed
```

Next wait for the ProcessManager to resume the application thread:

```
(gdb) sel4 switch-symbols process_manager
Reading symbols from out/cantrip/riscv32-unknown-elf/debug/process_manager.instance.bin...
(gdb) sel4 break process_manager_process_manager_proc_ctrl_0000_tcb cantrip_proc_manager::sel4bundle::{impl#1}::resume
(gdb) sel4 list-breakpoints
----------------------------------------------------------------------
|Thread                                            |Address|Temporary|
----------------------------------------------------------------------
|process_manager_process_manager_proc_ctrl_0000_tcb|0x3FDEE|False    |
----------------------------------------------------------------------
(gdb) c
Continuing.
```

Then kickoff the application from the console shell:
```
CANTRIP> start keyval
Bundle "keyval" started
```

When GDB stops at the breakpoint the thread will be known to seL4 and
you can use the usual seL4 gdb commands:

```
Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
[Switching to Thread 2]
cantrip_proc_manager::sel4bundle::{impl#1}::resume (self=0x0) at cantrip-proc-manager/src/sel4bundle/mod.rs:725
725         fn resume(&self) -> Result<(), ProcessManagerError> {
(gdb) sel4 threads
...
keyval
(gdb) sel4 tbreak keyval
(gdb) sel4 list-breakpoints
----------------------------------------------------------------------
|Thread                                            |Address|Temporary|
----------------------------------------------------------------------
|process_manager_process_manager_proc_ctrl_0000_tcb|0x3FDEE|False    |
|keyval                                            |any    |True     |
----------------------------------------------------------------------
(gdb) c
Continuing.

At this point you are stopped in the application at the first instruction and you will
want to manually load the application's symbols:
```
Thread 2 "matcha.cpu1" received signal SIGTRAP, Trace/breakpoint trap.
0x00010770 in exit ()
(gdb) add-symbol-file /usr/local/google/home/sleffler/sparrow/out/cantrip/riscv32-unknown-elf/debug/apps/rust/keyval/keyval.elf
add symbol table from file "/usr/local/google/home/sleffler/sparrow/out/cantrip/riscv32-unknown-elf/debug/apps/rust/keyval/keyval.elf"
Reading symbols from /usr/local/google/home/sleffler/sparrow/out/cantrip/riscv32-unknown-elf/debug/apps/rust/keyval/keyval.elf...
```

From here on you should be able to work as normal. If there are
previously loaded symbols that conflict just remove them with gdb's
remove-symbol-file command (check the output of info file to see what
symbols gdb has loaded).  Note that several of the above steps requires
Renode to single-step execution which is slow.

Beware that the sel4 extension code uses substring matching of component
names for operations like `sel4 break`. If the application name is a
substring (e.g. "timer") it is not possible to qualify the name. For the
moment you will want to use a different / unique name.

[*eventually the above process should be automated to eliminate the complexity*]

## Esoterica

The Renode extensions identify seL4 threads by intercepting
`seL4_DebugNameThread` system calls that assign a thread an ASCII
string. Typically this happens when the thread first starts up but
it can also happen otherwise; e.g. when the rootserver constructs
a thread it may assign a name. This means that a command like `sel4
wait-for-thread <threadname>` may stop GDB in an unexpected context (e.g.
the rootserver).

In order for the Renode extension to intercept `seL4_DebugNameThread`
system calls it must know the system call ID. Because seL4 does not
have a fixed ABI this number may change depending on how the system is
built and/or if the system source is modified. To deal with this the
system call ID is specified as an argument to the `CreateSeL4` Renode
directive; e.g. `monitor cpu1 CreateSeL4 0xffffffee`.  If you notice seL4
threads are not identified and simulations slow to a crawl then check
the setting--when there is a msimatch Renode will be single-stepping
execution waiting for threads to be entered for the first time (the
`sel4 ready` command may also be helpful in diagnosing this behaviour).
