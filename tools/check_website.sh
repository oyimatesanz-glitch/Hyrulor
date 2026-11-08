#!/bin/sh
URL=$(echo "$1" | sed -n 's/.*"url": *"\([^"]*\)".*/\1/p'); [ -z "$URL" ] && URL="https://google.com"
curl -s -o /dev/null -w "Status HTTP: %{http_code}\nTiempo: %{time_total}s\n" "$URL"
