#!/bin/sh
uptime "$@" 2>&1 | head -n 50
