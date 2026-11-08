#!/bin/sh
cut "$@" 2>&1 | head -n 50
