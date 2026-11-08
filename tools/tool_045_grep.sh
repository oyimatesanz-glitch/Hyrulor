#!/bin/sh
grep "$@" 2>&1 | head -n 50
