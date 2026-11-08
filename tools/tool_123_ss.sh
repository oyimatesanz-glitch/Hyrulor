#!/bin/sh
ss -h "$@" 2>&1 | head -n 50
