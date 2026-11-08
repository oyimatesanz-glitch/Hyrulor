#!/bin/sh
nc -zv "$@" 2>&1 | head -n 50
