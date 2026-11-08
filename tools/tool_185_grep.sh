#!/bin/sh
grep -t "$@" 2>&1 | head -n 50
