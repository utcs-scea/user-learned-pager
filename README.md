# User Learned Pager

## Build Instructions

Just run `make all`, you may have to enable more pages for mmap with `sysctl` (this should happen automatically in the makefile).

## Ideas and Advice

Changing `src/perf/counter.c` will allow you to capture the counts you wanted.
using `scripts/perf_type_config.sh` should help you fill in the blanks on your machine.
THIS IS REQUIRED FOR ANYTHING THAT IS ARCH SPECIFIC.

## Examples

### Gups

This program outputs random garbage to standard output, so I suggest piping that to null.
Stats go to file descriptor 3.
Fault addresses go to standard error.

Gups Variant to check overheads

```
Gups Variant to check overheads

Usage: gups [OPTIONS] [FUNCTION_TYPE] > /dev/null 2> vfa.stats 3> tlb.stats

Arguments:
  [FUNCTION_TYPE]  Function that should be used [default: shift-xor] [possible values: shift-xor, phase-shifting]

Options:
  -s, --size-buffer <SIZE_BUFFER>    Size of buffer in bytes
  -n, --num-attempts <NUM_ATTEMPTS>  Number of times to request buffer
  -t, --timer                        Enable Timer Measurements
  -u, --usecs <USECS>                Microseconds of Timer Signal
  -d, --disable-thp                  Disable Transparent Huge Pages
  -h, --help                         Print help
  -V, --version                      Print version
```
