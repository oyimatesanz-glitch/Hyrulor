#!/bin/sh
find -l "$@" 2>&1 | head -n 50
