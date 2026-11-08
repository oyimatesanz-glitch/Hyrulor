#!/bin/sh
chown "$@" 2>&1 | head -n 50
