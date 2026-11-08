#!/bin/sh
grep -a "$@" 2>&1 | head -n 50
