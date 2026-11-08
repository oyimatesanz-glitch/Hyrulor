#!/bin/sh
docker -a "$@" 2>&1 | head -n 50
