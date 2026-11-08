#!/bin/sh
host "$@" 2>&1 | head -n 50
