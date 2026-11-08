#!/bin/sh
ufw -a "$@" 2>&1 | head -n 50
