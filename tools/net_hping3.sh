#!/bin/sh
hping3 -c 1 "$@" 2>&1 | head -n 50
