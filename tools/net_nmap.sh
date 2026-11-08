#!/bin/sh
nmap -F "$@" 2>&1 | head -n 50
