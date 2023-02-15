
## Testing support

CantripOS testing support is a work in progress. Tests break down into
the following areas:

- *unit tests*: Rust cargo unit tests
- *sel4test*: seL4's sel4test framework
- *shell tests*: tests built into the CantripOS shell
- *application tests*: test applications (mostly to exercise the SDK)
- *robot tests*: automated tests that leverage the CantripOS shell

### Unit tests

Unit tests excercise functional interfaces with tests that run on the
build system (aka the "host").
These are all cargo-based and meant to be fast enough to run as part of
a pre-submit process.
The available tests can be found with the hmm command or using tab
completion:

``` shell
$ hmm cargo_test_<TAB>
cargo_test_cantrip                           cargo_test_cantrip_os_common_logger          cargo_test_cantrip_os_common_slot_allocator
cargo_test_cantrip_proc_interface            cargo_test_cantrip_proc_manager              cargo_test_debugconsole_zmodem
$ m cargo_test_cantrip
   ...
   Compiling memchr v2.5.0
   ...

running 5 tests
test tests::test_each_log_level_works ... ok
test tests::test_embedded_nul ... ok
test tests::test_formatting ... ok
test tests::test_max_log_level ... ok
test tests::test_too_long ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
...

```
The main impediment to unit tests is structuring code so that
platform-independent code can be exercised in an isolated/host environment.
Expect the set of unit tests to grow as more code is structured with
unit testing in mind.

### sel4test

sel4test is the seL4 kernel test facility that substitutes a test
harness for the usual rootserver and then automatically runs a suite
of tests that exercises system call api's and checks operational
correctness.
This facility is supported with two make targets:

``` shell
$ hmm sel4test

sel4test: (defined in build/platforms/sparrow/sim_sel4test.mk)
 C-based libsel4 syscall api wrappers. The result is run under Renode.

$ hmm sel4test+wrapper

sel4test+wrapper: (defined in build/platforms/sparrow/sim_sel4test.mk)
 crate wrapped with C shims. The result is run under Renode.
```

The first command runs the upstream seL4 test mechanism unchanged;
this mostly verifies the CantripOS kernel (which follows upstream
seL4 but has some non-trivial changes).
The second command runs the upstream test mechanism but using the Rust sel4-sys
crate with wrappers around the Rust implementations for use by C code;
this is mostly intended to exercise sel4-sys.

Note the sel4test target uses a debug build; this is consistent with
how upstream works. The sel4test+wrapper target however uses a release build of
the user mode pieces to reduce the space overhead of the Rust wrappers.

Not all target platforms may support the above make targets.

### Shell tests

Shell tests refers to builtin commands in the DebugConsole that exercise parts
of the system.
By convention these have a "test_" prefix; e.g.

``` shell
CANTRIP> ?
...
test_alloc
test_alloc_error
test_mailbox
test_malloc
test_mfree
test_mlcancel
test_mlexecute
test_mlperiodic
test_obj_alloc
test_panic
test_timer_async
test_timer_blocking
test_timer_completed
CANTRIP> test_obj_alloc
32 bytes in-use, 63639264 bytes free, 32 bytes requested, 3342336 overhead
2 objs in-use, 2 objs requested
32 bytes in-use, 63639264 bytes free, 13648 bytes requested, 3342336 overhead
2 objs in-use, 11 objs requested
Batch alloc ok: ObjDescBundle { cnode: 43, depth: 7, objs: [ObjDesc { type_: seL4_TCBObject, count: 1, cptr: 0 }, ObjDesc { type_: seL4_EndpointObject, count: 2, cptr: 1 }, ObjDesc { type_: seL4_ReplyObject, count: 2, cptr: 3 }, ObjDesc { type_: seL4_SchedContextObject, count: 8, cptr: 5 }, ObjDesc { type_: seL4_RISCV_4K_Page, count: 10, cptr: 6 }] }
cantrip_object_alloc_in_cnode ok: ObjDescBundle { cnode: 43, depth: 5, objs: [ObjDesc { type_: seL4_TCBObject, count: 1, cptr: 0 }, ObjDesc { type_: seL4_EndpointObject, count: 1, cptr: 1 }, ObjDesc { type_: seL4_ReplyObject, count: 1, cptr: 2 }, ObjDesc { type_: seL4_SchedContextObject, count: 8, cptr: 3 }, ObjDesc { type_: seL4_RISCV_4K_Page, count: 2, cptr: 4 }] }
All tests passed!
```

Shell commands bloat a system image so are conditionally compiled in
(in particular release builds do not include any test commands).
Check `DebugConsole/cantrip-shell/Cargo.toml` for features named "TEST_*".
These control the set of test commands, some of which are platform-dependent.
Beware that some builtin tests may generate assertions that will kill the
console shell; e.g.

``` shell
CANTRIP> test_panic
panic::panicked at 'testing', cantrip-shell/src/test_panic.rs:34:5
```

### Application tests

There are several applications designed to exercise/test the SDKRuntime.
These are typically included in the builtins archive baked into a system image.
For example,

``` shell
CANTRIP> builtins
fibonacci.app 30852
hello.app 580
keyval.app 31040
logtest.app 26100
mltest.app 29832
mobilenet_v1_emitc_static.model 1001090
panic.app 24812
suicide.app 551
timer.app 3121
CANTRIP> install logtest.app
Collected 26100 bytes of data, crc32 8673c4b7
Application "logtest" installed
CANTRIP> start logtest
Bundle "logtest" started.
CANTRIP> [logtest]::logtest::ping!
[logtest]::DONE
stop logtest
Bundle "logtest" stopped.
CANTRIP> uninstall logtest
Bundle "logtest" uninstalled.
```

Unlike a shell builtin an application that dies can just be stopped and unloaded.
Note multiple applications can be run simultaneously (depending on available
resources) to exercise concurrent use of the SDKRuntime.
For example,

``` shell
CANTRIP> install timer.app
Collected 31212 bytes of data, crc32 8d3381c0
Application "timer" installed
CANTRIP> start fibonacci
Bundle "fibonacci" started.
CANTRIP> [fibonacci]::fibonacci::[ 0]                    0  0
[fibonacci]::fibonacci::[ 1]                    1  100
[fibonacci]::fibonacci::[ 2]                    1  200
s[fibonacci]::fibonacci::[ 3]                    2  300
t[fibonacci]::fibonacci::[ 4]                    3  400
ar[fibonacci]::fibonacci::[ 5]                    5  500
t [fibonacci]::fibonacci::[ 6]                    8  600
tim[fibonacci]::fibonacci::[ 7]                   13  700
er[fibonacci]::fibonacci::[ 8]                   21  800

[fibonacci]::fibonacci::[ 9]                   34  900
...
Bundle "timer" started.
CANTRIP> [fibonacci]::fibonacci::[26]               121393  2600
[timer]::timer::sdk_timer_cancel returned Err(SDKInvalidTimer) with nothing running
[timer]::timer::sdk_timer_poll returned Ok(0) with nothing running
[timer]::timer::sdk_timer_oneshot returned Err(SDKNoSuchTimer) with an invalid timer id
[timer]::timer::Timer 0 started
[fibonacci]::fibonacci::[27]               196418  2700
[fibonacci]::fibonacci::[28]               317811  2800
[timer]::timer::Timer 0 completed
[timer]::timer::Timer 1 started
[fibonacci]::fibonacci::[29]               514229  2900

```

By default, platforms without an interactive shell include an
`autostart.repl` script in their builtins bundle that runs the available applications.
Systems that have an interactive command line have a *builtins.repl* file that does
the same thing and can be run with the "source" command; e.g.

```shell
CANTRIP> builtins
builtins.repl 642
...
CANTRIP> source builtins.repl
CANTRIP> install hello.app
Collected 640 bytes of data, crc32 877a95c1
Application "hello" installed
...
```

### Robot tests

Robot tests refer to system-level tests that treat the system as a black box.
Typically these are automated and used for regression testing.
The sel4test mechanism can be used for this purpose as can many of the shell
and application tests described above.
The scripts we use with [Renode's Robot framework](https://renode.readthedocs.io/en/latest/introduction/testing.html)
are included in the *sim/tests* directory.


### [Next Section: Memory Footprint](MemoryFootprint.md)
