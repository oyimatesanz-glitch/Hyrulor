#!/bin/sh
zip -r "$@" 2>&1 | head -n 50
