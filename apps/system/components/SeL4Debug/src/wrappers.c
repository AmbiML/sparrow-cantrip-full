#include <camkes.h>
#include <sel4/syscalls.h>

void sel4debug_put_string(const char* msg) {
#ifdef CONFIG_PRINTING
  char buf[512];
  snprintf(buf, sizeof(buf), "%s\n", msg);
  seL4_DebugPutString(buf);
#endif
}

void sel4debug_dump_scheduler() {
#ifdef CONFIG_PRINTING
  seL4_DebugDumpScheduler();
#endif
}
