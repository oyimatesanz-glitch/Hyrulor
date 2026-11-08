#!/bin/sh
lsblk "$@" 2>&1 | head -n 50
