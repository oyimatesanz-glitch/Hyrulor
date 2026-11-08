#!/bin/sh
git -v "$@" 2>&1 | head -n 50
