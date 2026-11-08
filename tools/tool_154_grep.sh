#!/bin/sh
grep --all "$@" 2>&1 | head -n 50
