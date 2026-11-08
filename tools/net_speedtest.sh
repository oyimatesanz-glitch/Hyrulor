#!/bin/sh
speedtest-cli "$@" 2>&1 | head -n 50
