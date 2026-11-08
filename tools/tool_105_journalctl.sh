#!/bin/sh
journalctl --verbose "$@" 2>&1 | head -n 50
