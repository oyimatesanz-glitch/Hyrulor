#!/bin/sh
nslookup "$@" 2>&1 | head -n 50
