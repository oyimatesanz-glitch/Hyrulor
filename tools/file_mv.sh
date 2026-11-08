#!/bin/sh
mv "$@" 2>&1 | head -n 50
