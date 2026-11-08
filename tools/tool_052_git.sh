#!/bin/sh
git -h "$@" 2>&1 | head -n 50
