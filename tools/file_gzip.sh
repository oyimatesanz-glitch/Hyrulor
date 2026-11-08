#!/bin/sh
gzip "$@" 2>&1 | head -n 50
