#!/usr/bin/env bash
echo "{\"gpus\": ["
lspci | grep -iE "vga|3d|display" | while read line; do
    echo "  \"$line\","
done | sed '$ s/,$//'
echo "]}"
