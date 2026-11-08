#!/bin/sh
scp -r "$@" 2>&1 | head -n 50
