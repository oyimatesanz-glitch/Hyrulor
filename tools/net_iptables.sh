#!/bin/sh
iptables -L "$@" 2>&1 | head -n 50
