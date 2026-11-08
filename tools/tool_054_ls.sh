#!/bin/sh
ls -v "$@" 2>&1 | head -n 50
