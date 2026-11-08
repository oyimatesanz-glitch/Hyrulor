#!/bin/sh
cat -l "$@" 2>&1 | head -n 50
