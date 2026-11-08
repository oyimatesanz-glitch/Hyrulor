#!/bin/sh
ufw -t "$@" 2>&1 | head -n 50
