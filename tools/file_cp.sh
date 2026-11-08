#!/bin/sh
cp -r "$@" 2>&1 | head -n 50
