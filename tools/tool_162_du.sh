#!/bin/sh
du -v "$@" 2>&1 | head -n 50
