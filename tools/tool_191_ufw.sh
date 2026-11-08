#!/bin/sh
ufw -n 10 "$@" 2>&1 | head -n 50
