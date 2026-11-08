#!/bin/sh
ls -r "$@" 2>&1 | head -n 50
