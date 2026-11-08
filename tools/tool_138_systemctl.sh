#!/bin/sh
systemctl -h "$@" 2>&1 | head -n 50
