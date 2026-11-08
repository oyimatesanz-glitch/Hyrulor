#!/bin/sh
ip -t "$@" 2>&1 | head -n 50
