#!/bin/sh
find . -type f "$@" 2>&1 | head -n 50
