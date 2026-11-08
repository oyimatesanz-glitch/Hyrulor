#!/bin/sh
find --verbose "$@" 2>&1 | head -n 50
