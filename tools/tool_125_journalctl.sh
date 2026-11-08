#!/bin/sh
journalctl --all "$@" 2>&1 | head -n 50
