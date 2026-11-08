#!/bin/sh
curl -I "$@" 2>&1 | head -n 50
