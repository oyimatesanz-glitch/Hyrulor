#!/bin/sh
ss "$@" 2>&1 | head -n 50
