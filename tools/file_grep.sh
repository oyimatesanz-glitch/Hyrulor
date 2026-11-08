#!/bin/sh
grep -r "$@" 2>&1 | head -n 50
