#!/bin/sh
tar -czvf "$@" 2>&1 | head -n 50
