#!/bin/sh
unzip "$@" 2>&1 | head -n 50
