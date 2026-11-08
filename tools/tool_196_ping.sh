#!/bin/sh
ping --all "$@" 2>&1 | head -n 50
