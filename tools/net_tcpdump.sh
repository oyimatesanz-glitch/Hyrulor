#!/bin/sh
tcpdump -i any -c 5 "$@" 2>&1 | head -n 50
