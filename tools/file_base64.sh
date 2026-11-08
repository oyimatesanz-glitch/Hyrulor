#!/bin/sh
base64 "$@" 2>&1 | head -n 50
