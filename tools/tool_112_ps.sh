#!/bin/sh
ps --all "$@" 2>&1 | head -n 50
