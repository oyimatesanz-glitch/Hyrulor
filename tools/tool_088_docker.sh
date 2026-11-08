#!/bin/sh
docker -v "$@" 2>&1 | head -n 50
