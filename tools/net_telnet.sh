#!/bin/sh
telnet "$@" 2>&1 | head -n 50
