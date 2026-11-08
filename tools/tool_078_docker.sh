#!/bin/sh
docker -r "$@" 2>&1 | head -n 50
