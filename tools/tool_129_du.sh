#!/bin/sh
du -h "$@" 2>&1 | head -n 50
