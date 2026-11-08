#!/bin/sh
URL=$(echo "$1" | sed -n 's/.*"url": *"\([^"]*\)".*/\1/p')
if [ -z "$URL" ]; then echo "Error: falta url"; exit 1; fi
curl -s -L --max-time 5 "$URL" | head -c 1000
