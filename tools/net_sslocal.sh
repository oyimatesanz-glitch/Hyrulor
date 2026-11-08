#!/bin/sh
sslocal -c "$@" 2>&1 | head -n 50
