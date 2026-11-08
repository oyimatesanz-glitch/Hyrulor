#!/bin/sh
cat -a "$@" 2>&1 | head -n 50
