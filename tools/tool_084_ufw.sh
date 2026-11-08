#!/bin/sh
ufw -r "$@" 2>&1 | head -n 50
