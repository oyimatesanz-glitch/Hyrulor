#!/bin/sh
stat "$@" 2>&1 | head -n 50
