#!/bin/sh
ip -a "$@" 2>&1 | head -n 50
