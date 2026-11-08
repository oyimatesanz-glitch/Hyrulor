#!/bin/sh
journalctl -v "$@" 2>&1 | head -n 50
