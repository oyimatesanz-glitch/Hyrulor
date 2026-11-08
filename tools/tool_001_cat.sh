#!/bin/sh
cat -t "$@" 2>&1 | head -n 50
