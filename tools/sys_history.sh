#!/bin/sh
history "$@" 2>&1 | head -n 50
