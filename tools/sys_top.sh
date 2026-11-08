#!/bin/sh
top -b -n 1 | head -n 15 "$@" 2>&1 | head -n 50
