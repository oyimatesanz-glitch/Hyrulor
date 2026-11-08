#!/bin/sh
find -h "$@" 2>&1 | head -n 50
