#!/bin/sh
wrk -t1 -c1 -d1s "$@" 2>&1 | head -n 50
