#!/bin/sh
git --verbose "$@" 2>&1 | head -n 50
