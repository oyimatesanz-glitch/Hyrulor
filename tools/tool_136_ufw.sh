#!/bin/sh
ufw --verbose "$@" 2>&1 | head -n 50
