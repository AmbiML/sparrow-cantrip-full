#include <camkes.h>
#include <sel4/syscalls.h>

void sel4debug_put_string(const char* msg) {
#ifdef CONFIG_PRINTING
  seL4_DebugPutString((char*) msg);
#endif
}

void sel4debug_dump_scheduler() {
#ifdef CONFIG_PRINTING
  seL4_DebugDumpScheduler();
#endif
}
