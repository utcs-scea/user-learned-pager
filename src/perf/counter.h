#include <stdint.h>

typedef struct counter {
  uint32_t type;
  uint64_t config;
  char* name;
} counter;

extern counter* counters;

extern uint8_t num_counters;
