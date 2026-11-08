#!/bin/sh
journalctl "$@" 2>&1 | head -n 50
