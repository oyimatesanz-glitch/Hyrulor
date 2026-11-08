#!/bin/sh
find -r "$@" 2>&1 | head -n 50
