#!/bin/sh
ls -l "$@" 2>&1 | head -n 50
