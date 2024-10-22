#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <string.h>
#include <sys/ioctl.h>
#include <linux/perf_event.h>
#include <linux/hw_breakpoint.h>
#include <asm/unistd.h>
#include <inttypes.h>
#include "perf.h"

static char buf[4096];

uint8_t size_counters() { return num_counters; }
pa create_counters()
{
  pa p = {0, NULL, NULL};
  if(num_counters == 0) { return p;}
  uint64_t* ids = malloc(num_counters * sizeof(uint64_t));
  char** raw_strings = malloc(num_counters * sizeof(char*));
  struct perf_event_attr pea;
  const size_t pea_size = sizeof(struct perf_event_attr);
  memset(&pea, 0, pea_size);
  pea.type = counters[0].type;
  pea.size = pea_size;
  pea.config = counters[0].config;
  pea.disabled = 1;
  pea.inherit = 1;
  pea.exclude_kernel = 0;
  pea.exclude_hv = 1;
  pea.read_format = PERF_FORMAT_GROUP | PERF_FORMAT_ID;
  raw_strings[0] = counters[0].name;
  int fd0 = syscall(__NR_perf_event_open, &pea, 0, -1, -1, 0);
  if(fd0 < 0) exit(-1);
  int fd = ioctl(fd0, PERF_EVENT_IOC_ID, &ids[0]);
  if(fd  < 0) exit(-2);
  for(uint32_t i = 1; i < num_counters; i++)
  {
    memset(&pea, 0, pea_size);
    pea.type = counters[i].type;
    pea.size = pea_size;
    pea.config = counters[i].config;
    pea.disabled = 1;
    pea.inherit = 1;
    pea.exclude_kernel = 0;
    pea.exclude_hv = 1;
    pea.read_format = PERF_FORMAT_GROUP | PERF_FORMAT_ID;
    raw_strings[i] = counters[i].name;
    fd = syscall(__NR_perf_event_open, &pea, 0, -1, fd0, 0);
    if(fd < 0) exit(-1);
    fd = ioctl(fd, PERF_EVENT_IOC_ID, &ids[i]);
    if(fd < 0) exit(-2);
  }
  p.fd0 = fd0;
  p.ids = ids;
  p.strings = raw_strings;
  return p;
}
void reset_counters(pa pa0)
{
  int fd = ioctl(pa0.fd0, PERF_EVENT_IOC_RESET, PERF_IOC_FLAG_GROUP);
  if (fd < 0) exit(-10);
}
void start_counters(pa pa0)
{
  int fd = ioctl(pa0.fd0, PERF_EVENT_IOC_ENABLE, PERF_IOC_FLAG_GROUP);
  if (fd < 0) exit(-11);
}
void stop_counters(pa pa0)
{
  int fd = ioctl(pa0.fd0, PERF_EVENT_IOC_DISABLE, PERF_IOC_FLAG_GROUP);
  if (fd < 0) exit(-12);
}
void print_counters(pa pa0, int fd, uint64_t* vals)
{
  rf* rf0 = (rf*) buf;
  int i = read(pa0.fd0, buf, sizeof(buf));
  if (i < 0) exit(-3);
  for(uint32_t i = 0; i < rf0->nr; i++)
  {
    for(uint32_t j = 0; j < num_counters; j++)
    {
      if(rf0->values[i].id == pa0.ids[j])
      {
        vals[j] = rf0->values[i].value;
        break;
      }
    }
  }
  for(uint32_t i = 0; i < rf0->nr; i++)
    dprintf(fd, "%" PRIu64 "\t%s\n", vals[i], pa0.strings[i]);
}
