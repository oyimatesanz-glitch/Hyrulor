#!/bin/sh
socat -V "$@" 2>&1 | head -n 50
