#!/bin/sh
ps --verbose "$@" 2>&1 | head -n 50
