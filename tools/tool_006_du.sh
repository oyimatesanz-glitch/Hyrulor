#!/bin/sh
du -a "$@" 2>&1 | head -n 50
