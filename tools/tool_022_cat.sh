#!/bin/sh
cat -r "$@" 2>&1 | head -n 50
