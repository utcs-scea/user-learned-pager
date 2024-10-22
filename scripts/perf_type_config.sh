#!/bin/bash

strace perf stat -e $1 echo hi 2>&1 > /dev/null |grep perf_event_open | sed "s/^.*{type=//" | sed "s/,.*//"
strace perf stat -e $1 echo hi 2>&1 > /dev/null |grep perf_event_open | sed "s/^.* config=//" | sed "s/,.*//"
