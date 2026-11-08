#!/bin/sh
du --verbose "$@" 2>&1 | head -n 50
