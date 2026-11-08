#!/bin/sh
systemctl -r "$@" 2>&1 | head -n 50
