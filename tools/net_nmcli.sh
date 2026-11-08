#!/bin/sh
nmcli dev status "$@" 2>&1 | head -n 50
