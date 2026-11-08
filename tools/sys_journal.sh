#!/bin/sh
journalctl -n 20 --no-pager "$@" 2>&1 | head -n 50
