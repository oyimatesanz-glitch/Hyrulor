#!/bin/sh
dmesg | tail -n 20 "$@" 2>&1 | head -n 50
