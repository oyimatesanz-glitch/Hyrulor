#!/bin/sh
siege -c 1 -r 1 "$@" 2>&1 | head -n 50
