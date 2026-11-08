#!/bin/sh
chmod +x "$@" 2>&1 | head -n 50
