#!/bin/sh
pidof "$@" 2>&1 | head -n 50
