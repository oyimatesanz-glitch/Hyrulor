#!/bin/sh
ab -n 10 -c 1 "$@" 2>&1 | head -n 50
