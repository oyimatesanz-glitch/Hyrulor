#!/bin/sh
wc --all "$@" 2>&1 | head -n 50
