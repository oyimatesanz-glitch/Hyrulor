#!/bin/sh
docker -h "$@" 2>&1 | head -n 50
