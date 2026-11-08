#!/bin/sh
firewall-cmd --list-all "$@" 2>&1 | head -n 50
