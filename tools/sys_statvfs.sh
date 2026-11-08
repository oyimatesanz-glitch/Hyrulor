#!/bin/sh
stat -f "$@" 2>&1 | head -n 50
