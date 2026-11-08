#!/bin/sh
curl -r "$@" 2>&1 | head -n 50
