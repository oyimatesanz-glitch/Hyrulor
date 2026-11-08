#!/bin/sh
uniq "$@" 2>&1 | head -n 50
