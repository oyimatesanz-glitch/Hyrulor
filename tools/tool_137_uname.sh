#!/bin/sh
uname --all "$@" 2>&1 | head -n 50
