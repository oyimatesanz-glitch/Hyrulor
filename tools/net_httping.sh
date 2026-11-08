#!/bin/sh
httping "$@" 2>&1 | head -n 50
