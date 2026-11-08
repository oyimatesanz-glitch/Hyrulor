#!/bin/sh
systemctl --verbose "$@" 2>&1 | head -n 50
