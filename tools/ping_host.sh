#!/bin/sh
ping -c 2 "$@" 2>&1 | head -n 50
