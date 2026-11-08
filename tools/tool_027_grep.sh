#!/bin/sh
grep -v "$@" 2>&1 | head -n 50
