#!/bin/sh
grep -l "$@" 2>&1 | head -n 50
