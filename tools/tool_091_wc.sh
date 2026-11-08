#!/bin/sh
wc "$@" 2>&1 | head -n 50
