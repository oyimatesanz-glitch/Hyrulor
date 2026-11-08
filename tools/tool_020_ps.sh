#!/bin/sh
ps -l "$@" 2>&1 | head -n 50
