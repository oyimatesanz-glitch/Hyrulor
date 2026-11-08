#!/bin/sh
touch "$@" 2>&1 | head -n 50
