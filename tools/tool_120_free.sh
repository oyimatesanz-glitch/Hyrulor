#!/bin/sh
free -v "$@" 2>&1 | head -n 50
