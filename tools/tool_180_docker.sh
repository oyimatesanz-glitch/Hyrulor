#!/bin/sh
docker -t "$@" 2>&1 | head -n 50
