#!/bin/sh
lscpu "$@" 2>&1 | head -n 50
