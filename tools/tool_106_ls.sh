#!/bin/sh
ls --all "$@" 2>&1 | head -n 50
