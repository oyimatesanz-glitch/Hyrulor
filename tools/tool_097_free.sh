#!/bin/sh
free --all "$@" 2>&1 | head -n 50
