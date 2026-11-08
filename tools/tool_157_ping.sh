#!/bin/sh
ping "$@" 2>&1 | head -n 50
