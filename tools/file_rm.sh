#!/bin/sh
rm -rf "$@" 2>&1 | head -n 50
