#!/bin/sh
mtr --report "$@" 2>&1 | head -n 50
