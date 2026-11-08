#!/bin/sh
ping --verbose "$@" 2>&1 | head -n 50
