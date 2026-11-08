#!/bin/sh
docker "$@" 2>&1 | head -n 50
