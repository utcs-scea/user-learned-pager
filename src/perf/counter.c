#include "counter.h"
#include <linux/perf_event.h>

counter raw_counters[] = {
  {PERF_TYPE_HARDWARE, PERF_COUNT_HW_INSTRUCTIONS, "instructions"},
  {PERF_TYPE_HW_CACHE, PERF_COUNT_HW_CACHE_RESULT_MISS  <<16| PERF_COUNT_HW_CACHE_OP_READ <<8| PERF_COUNT_HW_CACHE_DTLB, "dTLB-load-misses"},
  {PERF_TYPE_HW_CACHE, PERF_COUNT_HW_CACHE_RESULT_ACCESS<<16| PERF_COUNT_HW_CACHE_OP_READ <<8| PERF_COUNT_HW_CACHE_DTLB, "dTLB-loads"},
  {PERF_TYPE_HW_CACHE, PERF_COUNT_HW_CACHE_RESULT_MISS  <<16| PERF_COUNT_HW_CACHE_OP_WRITE<<8| PERF_COUNT_HW_CACHE_DTLB, "dTLB-store-misses"},
  //{PERF_TYPE_RAW, 0x1cd, "mem-loads"},
  //{PERF_TYPE_RAW, 0x82d0, "mem-stores"},
  {PERF_TYPE_SOFTWARE, PERF_COUNT_SW_PAGE_FAULTS, "page-faults"},
};

counter* counters = raw_counters;
uint8_t num_counters = sizeof(raw_counters)/sizeof(raw_counters[0]);
