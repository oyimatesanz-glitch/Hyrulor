#!/bin/sh
git --all "$@" 2>&1 | head -n 50
