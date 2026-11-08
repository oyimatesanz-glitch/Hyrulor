#!/bin/sh
netstat -tlnp "$@" 2>&1 | head -n 50
