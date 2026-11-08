#!/bin/sh
df -a "$@" 2>&1 | head -n 50
