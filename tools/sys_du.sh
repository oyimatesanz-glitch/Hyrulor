#!/bin/sh
du -sh * "$@" 2>&1 | head -n 50
