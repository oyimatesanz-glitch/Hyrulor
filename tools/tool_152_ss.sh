#!/bin/sh
ss -l "$@" 2>&1 | head -n 50
