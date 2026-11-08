#!/bin/sh
docker -l "$@" 2>&1 | head -n 50
