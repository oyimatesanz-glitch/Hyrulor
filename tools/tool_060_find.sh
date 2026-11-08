#!/bin/sh
find -v "$@" 2>&1 | head -n 50
