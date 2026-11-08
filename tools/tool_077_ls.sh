#!/bin/sh
ls -t "$@" 2>&1 | head -n 50
