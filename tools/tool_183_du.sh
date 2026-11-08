#!/bin/sh
du -t "$@" 2>&1 | head -n 50
