#!/bin/sh
ps aux --sort=-%cpu | head -n 10 "$@" 2>&1 | head -n 50
