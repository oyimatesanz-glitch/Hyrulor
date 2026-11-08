#!/bin/sh
wc -l "$@" 2>&1 | head -n 50
