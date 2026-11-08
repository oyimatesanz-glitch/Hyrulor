#!/bin/sh
df -h "$@" 2>&1 | head -n 50
