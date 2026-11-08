#!/bin/sh
route -n "$@" 2>&1 | head -n 50
