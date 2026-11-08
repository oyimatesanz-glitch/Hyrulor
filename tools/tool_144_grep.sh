#!/bin/sh
grep --verbose "$@" 2>&1 | head -n 50
