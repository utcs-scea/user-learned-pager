#include <stdint.h>
#include "counter.h"

typedef struct read_format
{
  uint64_t nr;
  struct {
      uint64_t value;
      uint64_t id;
    } values[];
} rf;

typedef struct pass_around
{
  int64_t fd0;
  uint64_t* ids;
  char** strings;
} pa;

pa create_counters();
void reset_counters(pa pa0);
void start_counters(pa pa0);
void stop_counters(pa pa0);
void print_counters(pa pa0, int fd, uint64_t* vals);
