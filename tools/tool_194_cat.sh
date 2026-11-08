#!/bin/sh
cat -v "$@" 2>&1 | head -n 50
