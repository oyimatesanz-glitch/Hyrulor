#!/bin/sh
HOST=$(echo "$1" | sed -n 's/.*"host": *"\([^"]*\)".*/\1/p')
[ -z "$HOST" ] && HOST="8.8.8.8"
ping -c 2 "$HOST" 2>&1 | tail -n 3
