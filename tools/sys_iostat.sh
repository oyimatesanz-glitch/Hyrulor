#!/bin/sh
iostat "$@" 2>&1 | head -n 50
