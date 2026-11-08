#!/bin/sh
curl --verbose "$@" 2>&1 | head -n 50
