#!/bin/sh
tty "$@" 2>&1 | head -n 50
