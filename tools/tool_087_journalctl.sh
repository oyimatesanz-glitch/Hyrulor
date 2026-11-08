#!/bin/sh
journalctl -l "$@" 2>&1 | head -n 50
