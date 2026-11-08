#!/bin/sh
systemctl -l "$@" 2>&1 | head -n 50
