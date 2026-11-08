#!/bin/sh
git -t "$@" 2>&1 | head -n 50
