#!/bin/sh
curl -t "$@" 2>&1 | head -n 50
