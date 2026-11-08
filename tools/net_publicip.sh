#!/bin/sh
curl -s ifconfig.me "$@" 2>&1 | head -n 50
