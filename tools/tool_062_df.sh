#!/bin/sh
df -v "$@" 2>&1 | head -n 50
