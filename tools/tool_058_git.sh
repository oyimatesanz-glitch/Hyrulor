#!/bin/sh
git -l "$@" 2>&1 | head -n 50
