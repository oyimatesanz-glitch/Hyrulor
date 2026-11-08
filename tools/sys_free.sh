#!/bin/sh
free -h "$@" 2>&1 | head -n 50
