#!/bin/sh
find -a "$@" 2>&1 | head -n 50
