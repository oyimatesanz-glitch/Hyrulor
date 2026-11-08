#!/bin/sh
wc -h "$@" 2>&1 | head -n 50
