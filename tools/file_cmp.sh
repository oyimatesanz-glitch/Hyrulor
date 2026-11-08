#!/bin/sh
cmp "$@" 2>&1 | head -n 50
