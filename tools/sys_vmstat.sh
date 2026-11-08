#!/bin/sh
vmstat "$@" 2>&1 | head -n 50
