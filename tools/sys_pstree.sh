#!/bin/sh
pstree -p "$@" 2>&1 | head -n 50
