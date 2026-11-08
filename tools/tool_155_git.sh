#!/bin/sh
git -r "$@" 2>&1 | head -n 50
