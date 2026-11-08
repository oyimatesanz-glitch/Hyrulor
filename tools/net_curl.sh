#!/bin/sh
curl -sL "$@" 2>&1 | head -n 50
