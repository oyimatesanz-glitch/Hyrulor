#!/bin/sh
cat --all "$@" 2>&1 | head -n 50
