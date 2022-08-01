#include <model/statedata.h>
#include <object/structures.h>

/* Dual-domain schedule for Cantrip to isolate third party applications from system
 * applications.
 *
 * Note that this doesn't actually implement the schedule -- that's hardwired in
 * seL4's kernel source. See also cantrip/kernel/src/kernel/thread.c, in the
 * nextDomain function around line 302 and the timerTick function around 630.
 *
 * Effectively this is a round-robin scheduler, so half of the CPU time is given
 * to system applications, while third party applications are allocated the
 * other half. Note that even if there's nothing to run in the third-party
 * application domain, the scheduler will schedule an idle thread to ensure that
 * domain gets it's allocated share of time.
 *
 * TODO(jtgans,sleffler): Figure out how to better use these domains for
 * scheduling applications. We don't really want to use a full 50% duty cycle
 * for third party applications -- this wastes too much time. See also
 * b/238811077.
 *
 * NOTE: Only a single domain is currently enabled, as per the commented section
 * below. Any time the below schedule is changed, the number of domains
 * configured in easy-settings.cmake must also be changed.
 */
const dschedule_t ksDomSchedule[] = {
    {.domain = 0, .length = 1},  // System domain
    /*  {.domain = 1, .length = 1},  // Third party application domain */
};

const word_t ksDomScheduleLength = sizeof(ksDomSchedule) / sizeof(dschedule_t);
