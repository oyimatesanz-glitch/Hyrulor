#!/bin/sh
builtin "$@" 2>&1 | head -n 50
