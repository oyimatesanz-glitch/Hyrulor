#!/bin/sh
wc --verbose "$@" 2>&1 | head -n 50
