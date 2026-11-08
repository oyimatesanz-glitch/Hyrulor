#!/bin/sh
ufw status "$@" 2>&1 | head -n 50
