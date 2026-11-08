#!/bin/sh
DOMAIN=$(echo "$1" | sed -n 's/.*"domain": *"\([^"]*\)".*/\1/p'); [ -z "$DOMAIN" ] && DOMAIN="google.com"
nslookup "$DOMAIN" 2>&1 | grep -A 2 "Name:"
