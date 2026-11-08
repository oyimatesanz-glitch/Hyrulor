#!/bin/sh
declare -F "$@" 2>&1 | head -n 50
