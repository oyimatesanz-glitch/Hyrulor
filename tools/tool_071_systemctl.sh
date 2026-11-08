#!/bin/sh
systemctl --all "$@" 2>&1 | head -n 50
