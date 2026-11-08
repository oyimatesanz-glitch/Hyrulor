#!/bin/sh
journalctl -t "$@" 2>&1 | head -n 50
