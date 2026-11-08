#!/bin/sh
free "$@" 2>&1 | head -n 50
