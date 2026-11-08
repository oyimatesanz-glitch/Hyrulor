#!/bin/sh
command -V "$@" 2>&1 | head -n 50
